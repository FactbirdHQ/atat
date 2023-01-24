use bbqueue::framed::FrameConsumer;
use embedded_hal_nb::{nb, serial};
use fugit::ExtU32;

use crate::error::{Error, Response};
use crate::helpers::LossyStr;
use crate::traits::{AtatClient, AtatCmd, AtatUrc};
use crate::Config;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum ClientState {
    Idle,
    AwaitingResponse,
}

/// Whether the AT client should block while waiting responses or return early.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Mode {
    /// The function call will wait as long as necessary to complete the operation
    Blocking,
    /// The function call will not wait at all to complete the operation, and only do what it can.
    NonBlocking,
    /// The function call will wait only up the max timeout of each command to complete the operation.
    Timeout,
}

/// Client responsible for handling send, receive and timeout from the
/// userfacing side. The client is decoupled from the ingress-manager through
/// some spsc queue consumers, where any received responses can be dequeued. The
/// Client also has an spsc producer, to allow signaling commands like
/// `reset` to the ingress-manager.
pub struct Client<
    Tx,
    CLK,
    const TIMER_HZ: u32,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
> where
    Tx: serial::Write<u8>,
    CLK: fugit_timer::Timer<TIMER_HZ>,
{
    /// Serial writer
    tx: Tx,

    /// The response consumer receives responses from the ingress manager
    res_c: FrameConsumer<'static, RES_CAPACITY>,
    /// The URC consumer receives URCs from the ingress manager
    urc_c: FrameConsumer<'static, URC_CAPACITY>,

    state: ClientState,
    timer: CLK,
    config: Config,
}

impl<Tx, CLK, const TIMER_HZ: u32, const RES_CAPACITY: usize, const URC_CAPACITY: usize>
    Client<Tx, CLK, TIMER_HZ, RES_CAPACITY, URC_CAPACITY>
where
    Tx: serial::Write<u8>,
    CLK: fugit_timer::Timer<TIMER_HZ>,
{
    pub fn new(
        tx: Tx,
        res_c: FrameConsumer<'static, RES_CAPACITY>,
        urc_c: FrameConsumer<'static, URC_CAPACITY>,
        mut timer: CLK,
        config: Config,
    ) -> Self {
        timer.start(config.cmd_cooldown.millis()).ok();

        Self {
            tx,
            res_c,
            urc_c,
            state: ClientState::Idle,
            config,
            timer,
        }
    }
}

/// Blocks on `nb` function calls but allow to time out
/// Example of usage:
/// `block_timeout!((client.timer, client.config.tx_timeout) => {client.tx.write(c)}.map_err(|_e| Error::Write))?;`
/// `block_timeout!((client.timer, client.config.tx_timeout) => {client.tx.write(c)});`
#[macro_export]
macro_rules! block_timeout {
    ($timer:expr, $duration:expr, $e:expr, $map_err:expr) => {{
        if $duration == 0 {
            nb::block!($e).map_err($map_err)
        } else {
            $timer.start($duration.millis()).ok();
            loop {
                match $e {
                    Err(nb::Error::WouldBlock) => (),
                    Err(nb::Error::Other(e)) => break Err($map_err(e)),
                    Ok(r) => break Ok(r),
                };
                match $timer.wait() {
                    Err(nb::Error::WouldBlock) => (),
                    Err(_) => break Err(Error::Write),
                    Ok(_) => break Err(Error::Timeout),
                };
            }
        }
    }};
    (($timer:expr, $duration:expr) => {$e:expr}) => {{
        block_timeout!($timer, $duration, $e, |e| { e })
    }};
    (($timer:expr, $duration:expr) => {$e:expr}.map_err($map_err:expr)) => {{
        block_timeout!($timer, $duration, $e, $map_err)
    }};
}

impl<Tx, CLK, const TIMER_HZ: u32, const RES_CAPACITY: usize, const URC_CAPACITY: usize> AtatClient
    for Client<Tx, CLK, TIMER_HZ, RES_CAPACITY, URC_CAPACITY>
where
    Tx: serial::Write<u8>,
    CLK: fugit_timer::Timer<TIMER_HZ>,
{
    fn send<A: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &A,
    ) -> nb::Result<A::Response, Error> {
        if self.state == ClientState::Idle {
            // compare the time of the last response or URC and ensure at least
            // `self.config.cmd_cooldown` ms have passed before sending a new
            // command
            nb::block!(self.timer.wait()).ok();
            let cmd_buf = cmd.as_bytes();

            if cmd_buf.len() < 50 {
                debug!("Sending command: \"{:?}\"", LossyStr(&cmd_buf));
            } else {
                debug!(
                    "Sending command with too long payload ({} bytes) to log!",
                    cmd_buf.len(),
                );
            }

            for c in cmd_buf {
                block_timeout!((self.timer, self.config.tx_timeout) => {self.tx.write(c)}.map_err(|_e| Error::Write))?;
            }
            block_timeout!((self.timer, self.config.flush_timeout) => {self.tx.flush()}.map_err(|_e| Error::Write))?;

            self.state = ClientState::AwaitingResponse;
        }

        if !A::EXPECTS_RESPONSE_CODE {
            self.state = ClientState::Idle;
            return cmd.parse(Ok(&[])).map_err(nb::Error::Other);
        }

        match self.config.mode {
            Mode::Blocking => Ok(nb::block!(self.check_response(cmd))?),
            Mode::NonBlocking => self.check_response(cmd),
            Mode::Timeout => {
                self.timer.start(A::MAX_TIMEOUT_MS.millis()).ok();
                Ok(nb::block!(self.check_response(cmd))?)
            }
        }
    }

    fn peek_urc_with<URC: AtatUrc, F: FnOnce(URC::Response) -> bool>(&mut self, f: F) {
        if let Some(urc_grant) = self.urc_c.read() {
            self.timer.start(self.config.cmd_cooldown.millis()).ok();
            if let Some(urc) = URC::parse(urc_grant.as_ref()) {
                if !f(urc) {
                    return;
                }
            } else {
                error!("Parsing URC FAILED: {:?}", LossyStr(urc_grant.as_ref()));
            }
            urc_grant.release();
        }
    }

    fn check_response<A: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &A,
    ) -> nb::Result<A::Response, Error> {
        if let Some(mut res_grant) = self.res_c.read() {
            res_grant.auto_release(true);

            let res = match Response::from(res_grant.as_ref()) {
                Response::Result(r) => r,
                Response::Prompt(_) => Ok(&[][..]),
            };

            return cmd
                .parse(res)
                .map_err(nb::Error::from)
                .and_then(|r| {
                    if self.state == ClientState::AwaitingResponse {
                        self.timer.start(self.config.cmd_cooldown.millis()).ok();
                        self.state = ClientState::Idle;
                        Ok(r)
                    } else {
                        Err(nb::Error::WouldBlock)
                    }
                })
                .map_err(|e| {
                    self.timer.start(self.config.cmd_cooldown.millis()).ok();
                    self.state = ClientState::Idle;
                    e
                });
        } else if self.config.mode == Mode::Timeout && self.timer.wait().is_ok() {
            self.state = ClientState::Idle;
            return Err(nb::Error::Other(Error::Timeout));
        }
        Err(nb::Error::WouldBlock)
    }

    fn get_mode(&self) -> Mode {
        self.config.mode
    }

    fn reset(&mut self) {
        while let Some(grant) = self.res_c.read() {
            grant.release();
        }

        while let Some(grant) = self.urc_c.read() {
            grant.release();
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::mpsc;
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::{self as atat, InternalError};
    use crate::{
        atat_derive::{AtatCmd, AtatEnum, AtatResp, AtatUrc},
        clock::Clock,
    };
    use bbqueue::framed::FrameProducer;
    use bbqueue::BBBuffer;
    use heapless::String;

    const TEST_RX_BUF_LEN: usize = 256;
    const TEST_RES_CAPACITY: usize = 3 * TEST_RX_BUF_LEN;
    const TEST_URC_CAPACITY: usize = 3 * TEST_RX_BUF_LEN;
    const TIMER_HZ: u32 = 1000;

    struct CdMock<const TIMER_HZ: u32> {
        handle: Option<JoinHandle<bool>>,
        trigger: Option<mpsc::Sender<bool>>,
    }
    impl CdMock<TIMER_HZ> {
        fn new() -> Self {
            CdMock {
                handle: None,
                trigger: None,
            }
        }
    }

    impl<const TIMER_HZ: u32> Clock<TIMER_HZ> for CdMock<TIMER_HZ> {
        type Error = core::convert::Infallible;

        /// Return current time `Instant`
        fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ> {
            fugit::TimerInstantU32::from_ticks(0)
        }

        /// Start countdown with a `duration`
        fn start(
            &mut self,
            duration: fugit::TimerDurationU32<TIMER_HZ>,
        ) -> Result<(), Self::Error> {
            let (tx, rx) = mpsc::channel();
            self.trigger = Some(tx.clone());

            thread::spawn(move || {
                let trigger = tx.clone();
                thread::sleep(Duration::from_millis(duration.to_millis() as u64));
                trigger.send(true).unwrap();
            });

            let handle = thread::spawn(move || loop {
                match rx.recv() {
                    Ok(ticked) => break ticked,
                    _ => break false,
                }
            });
            self.handle.replace(handle);
            Ok(())
        }

        /// Stop timer
        fn cancel(&mut self) -> Result<(), Self::Error> {
            match self.trigger.take() {
                Some(trigger) => Ok(trigger.send(false).unwrap()),
                None => Ok(()),
            }
        }

        /// Wait until countdown `duration` set with the `fn start` has expired
        fn wait(&mut self) -> nb::Result<(), Self::Error> {
            match &self.handle {
                Some(handle) => match handle.is_finished() {
                    true => self.handle = None,
                    false => Err(nb::Error::WouldBlock)?,
                },
                None => (),
            }
            Ok(())
        }
    }

    #[derive(Debug)]
    pub enum SerialError {}

    impl serial::Error for SerialError {
        fn kind(&self) -> serial::ErrorKind {
            serial::ErrorKind::Other
        }
    }

    struct TxMock {
        s: String<64>,
        timeout: Option<Duration>,
        timer_start: Option<Instant>,
    }

    impl TxMock {
        fn new(s: String<64>, timeout: Option<Duration>) -> Self {
            TxMock {
                s,
                timeout,
                timer_start: None,
            }
        }
    }

    impl serial::ErrorType for TxMock {
        type Error = serial::ErrorKind;
    }

    impl serial::Write<u8> for TxMock {
        fn write(&mut self, c: u8) -> nb::Result<(), Self::Error> {
            if let Some(timeout) = self.timeout {
                if self.timer_start.get_or_insert(Instant::now()).elapsed() < timeout {
                    return Err(nb::Error::WouldBlock);
                }
                self.timer_start.take();
            }
            self.s
                .push(c as char)
                .map_err(|_| nb::Error::Other(serial::ErrorKind::Other))
        }

        fn flush(&mut self) -> nb::Result<(), Self::Error> {
            if let Some(timeout) = self.timeout {
                if self.timer_start.get_or_insert(Instant::now()).elapsed() < timeout {
                    return Err(nb::Error::WouldBlock);
                }
                self.timer_start.take();
            }
            Ok(())
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum InnerError {
        Test,
    }

    impl core::str::FromStr for InnerError {
        // This error will always get mapped to `atat::Error::Parse`
        type Err = ();

        fn from_str(_s: &str) -> Result<Self, Self::Err> {
            Ok(Self::Test)
        }
    }

    #[derive(Debug, PartialEq, AtatCmd)]
    #[at_cmd("+CFUN", NoResponse, error = "InnerError")]
    struct ErrorTester {
        x: u8,
    }

    #[derive(Clone, AtatCmd)]
    #[at_cmd("+CFUN", NoResponse, timeout_ms = 180000)]
    pub struct SetModuleFunctionality {
        #[at_arg(position = 0)]
        pub fun: Functionality,
        #[at_arg(position = 1)]
        pub rst: Option<ResetMode>,
    }

    #[derive(Clone, AtatCmd)]
    #[at_cmd("+FUN", NoResponse, timeout_ms = 180000)]
    pub struct Test2Cmd {
        #[at_arg(position = 1)]
        pub fun: Functionality,
        #[at_arg(position = 0)]
        pub rst: Option<ResetMode>,
    }

    #[derive(Clone, AtatCmd)]
    #[at_cmd("+CUN", TestResponseString, timeout_ms = 180000)]
    pub struct TestRespStringCmd {
        #[at_arg(position = 0)]
        pub fun: Functionality,
        #[at_arg(position = 1)]
        pub rst: Option<ResetMode>,
    }
    #[derive(Clone, AtatCmd)]
    #[at_cmd("+CUN", TestResponseStringMixed, timeout_ms = 180000, attempts = 1)]
    pub struct TestRespStringMixCmd {
        #[at_arg(position = 1)]
        pub fun: Functionality,
        #[at_arg(position = 0)]
        pub rst: Option<ResetMode>,
    }

    // #[derive(Clone, AtatCmd)]
    // #[at_cmd("+CUN", TestResponseStringMixed, timeout_ms = 180000)]
    // pub struct TestUnnamedStruct(Functionality, Option<ResetMode>);

    #[derive(Clone, PartialEq, AtatEnum)]
    #[at_enum(u8)]
    pub enum Functionality {
        #[at_arg(value = 0)]
        Min,
        #[at_arg(value = 1)]
        Full,
        #[at_arg(value = 4)]
        APM,
        #[at_arg(value = 6)]
        DM,
    }

    #[derive(Clone, PartialEq, AtatEnum)]
    #[at_enum(u8)]
    pub enum ResetMode {
        #[at_arg(value = 0)]
        DontReset,
        #[at_arg(value = 1)]
        Reset,
    }
    #[derive(Clone, AtatResp, PartialEq, Debug)]
    pub struct NoResponse;

    #[derive(Clone, AtatResp, PartialEq, Debug)]
    pub struct TestResponseString {
        #[at_arg(position = 0)]
        pub socket: u8,
        #[at_arg(position = 1)]
        pub length: usize,
        #[at_arg(position = 2)]
        pub data: String<64>,
    }

    #[derive(Clone, AtatResp, PartialEq, Debug)]
    pub struct TestResponseStringMixed {
        #[at_arg(position = 1)]
        pub socket: u8,
        #[at_arg(position = 2)]
        pub length: usize,
        #[at_arg(position = 0)]
        pub data: String<64>,
    }

    #[derive(Debug, Clone, AtatResp, PartialEq)]
    pub struct MessageWaitingIndication {
        #[at_arg(position = 0)]
        pub status: u8,
        #[at_arg(position = 1)]
        pub code: u8,
    }

    #[derive(Debug, Clone, AtatUrc, PartialEq)]
    pub enum Urc {
        #[at_urc(b"+UMWI")]
        MessageWaitingIndication(MessageWaitingIndication),
        #[at_urc(b"CONNECT OK")]
        ConnectOk,
    }

    macro_rules! setup {
        ($config:expr => $timeout:expr) => {{
            static mut RES_Q: BBBuffer<TEST_RES_CAPACITY> = BBBuffer::new();
            let (res_p, res_c) = unsafe { RES_Q.try_split_framed().unwrap() };

            static mut URC_Q: BBBuffer<TEST_URC_CAPACITY> = BBBuffer::new();
            let (urc_p, urc_c) = unsafe { URC_Q.try_split_framed().unwrap() };

            let tx_mock = TxMock::new(String::new(), $timeout);
            let client: Client<
                TxMock,
                CdMock<TIMER_HZ>,
                TIMER_HZ,
                TEST_RES_CAPACITY,
                TEST_URC_CAPACITY,
            > = Client::new(tx_mock, res_c, urc_c, CdMock::new(), $config);
            (client, res_p, urc_p)
        }};
        ($config:expr, $timeout:expr) => {{
            setup!($config => Some($timeout))
        }};
        ($config:expr) => {{
            setup!($config => None)
        }};
    }

    pub fn enqueue_res(
        producer: &mut FrameProducer<'static, TEST_RES_CAPACITY>,
        res: Result<&[u8], InternalError>,
    ) {
        let header: crate::error::Encoded = res.into();

        let mut grant = producer.grant(header.len()).unwrap();
        match header {
            crate::error::Encoded::Simple(h) => grant[..1].copy_from_slice(&[h]),
            crate::error::Encoded::Nested(h, b) => {
                grant[..1].copy_from_slice(&[h]);
                grant[1..2].copy_from_slice(&[b]);
            }
            crate::error::Encoded::Array(h, b) => {
                grant[..1].copy_from_slice(&[h]);
                grant[1..header.len()].copy_from_slice(&b);
            }
            crate::error::Encoded::Slice(h, b) => {
                grant[..1].copy_from_slice(&[h]);
                grant[1..header.len()].copy_from_slice(b);
            }
        };
        grant.commit(header.len());
    }

    #[test]
    fn error_response() {
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking));

        let cmd = ErrorTester { x: 7 };

        enqueue_res(&mut p, Err(InternalError::Error));

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(nb::block!(client.send(&cmd)), Err(Error::Error));
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn generic_error_response() {
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking));

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        enqueue_res(&mut p, Err(InternalError::Error));

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(nb::block!(client.send(&cmd)), Err(Error::Error));
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn string_sent() {
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking));

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        enqueue_res(&mut p, Ok(&[]));

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.send(&cmd), Ok(NoResponse));
        assert_eq!(client.state, ClientState::Idle);

        assert_eq!(
            client.tx.s,
            String::<32>::from("AT+CFUN=4,0\r\n"),
            "Wrong encoding of string"
        );

        enqueue_res(&mut p, Ok(&[]));

        let cmd = Test2Cmd {
            fun: Functionality::DM,
            rst: Some(ResetMode::Reset),
        };
        assert_eq!(client.send(&cmd), Ok(NoResponse));

        assert_eq!(
            client.tx.s,
            String::<32>::from("AT+CFUN=4,0\r\nAT+FUN=1,6\r\n"),
            "Reverse order string did not match"
        );
    }

    #[test]
    fn blocking() {
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking));

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        enqueue_res(&mut p, Ok(&[]));

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.send(&cmd), Ok(NoResponse));
        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.tx.s, String::<32>::from("AT+CFUN=4,0\r\n"));
    }

    #[test]
    fn non_blocking() {
        let (mut client, mut p, _) = setup!(Config::new(Mode::NonBlocking));

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.send(&cmd), Err(nb::Error::WouldBlock));
        assert_eq!(client.state, ClientState::AwaitingResponse);

        assert_eq!(client.check_response(&cmd), Err(nb::Error::WouldBlock));

        enqueue_res(&mut p, Ok(&[]));

        assert_eq!(client.state, ClientState::AwaitingResponse);

        assert_eq!(client.check_response(&cmd), Ok(NoResponse));
        assert_eq!(client.state, ClientState::Idle);
    }

    // Test response containing string
    #[test]
    fn response_string() {
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking));

        // String last
        let cmd = TestRespStringCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let response = b"+CUN: 22,16,\"0123456789012345\"";
        enqueue_res(&mut p, Ok(response));

        assert_eq!(client.state, ClientState::Idle);

        assert_eq!(
            client.send(&cmd),
            Ok(TestResponseString {
                socket: 22,
                length: 16,
                data: String::<64>::from("0123456789012345")
            })
        );
        assert_eq!(client.state, ClientState::Idle);

        // Mixed order for string
        let cmd = TestRespStringMixCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let response = b"+CUN: \"0123456789012345\",22,16";
        enqueue_res(&mut p, Ok(response));

        assert_eq!(
            client.send(&cmd),
            Ok(TestResponseStringMixed {
                socket: 22,
                length: 16,
                data: String::<64>::from("0123456789012345")
            })
        );
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn urc() {
        let (mut client, _, mut urc_p) = setup!(Config::new(Mode::NonBlocking));

        let response = b"+UMWI: 0, 1";

        let mut grant = urc_p.grant(response.len()).unwrap();
        grant.copy_from_slice(response.as_ref());
        grant.commit(response.len());

        assert_eq!(client.state, ClientState::Idle);
        assert!(client.check_urc::<Urc>().is_some());
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn urc_keyword() {
        let (mut client, _, mut urc_p) = setup!(Config::new(Mode::NonBlocking));

        let response = b"CONNECT OK";

        let mut grant = urc_p.grant(response.len()).unwrap();
        grant.copy_from_slice(response.as_ref());
        grant.commit(response.len());

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(Urc::ConnectOk, client.check_urc::<Urc>().unwrap());
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn invalid_response() {
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking));

        // String last
        let cmd = TestRespStringCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let response = b"+CUN: 22,16,22";
        enqueue_res(&mut p, Ok(response));

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.send(&cmd), Err(nb::Error::Other(Error::Parse)));
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn tx_timeout() {
        let timeout = Duration::from_millis(20);
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking).tx_timeout(1), timeout);

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        enqueue_res(&mut p, Ok(&[]));

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.send(&cmd), Err(nb::Error::Other(Error::Timeout)));
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn flush_timeout() {
        let timeout = Duration::from_millis(20);
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking).flush_timeout(1), timeout);

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        enqueue_res(&mut p, Ok(&[]));

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.send(&cmd), Err(nb::Error::Other(Error::Timeout)));
        assert_eq!(client.state, ClientState::Idle);
    }
}
