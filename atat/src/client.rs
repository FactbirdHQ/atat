use bbqueue::framed::FrameConsumer;
use embedded_hal::serial;
use fugit::ExtU32;

use crate::error::Error;
use crate::helpers::LossyStr;
use crate::queues::ComProducer;
use crate::traits::{AtatClient, AtatCmd, AtatUrc};
use crate::ResponseHeader;
use crate::{Command, Config};

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
    Tx: serial::nb::Write<u8>,
    CLK: fugit_timer::Timer<TIMER_HZ>,
{
    /// Serial writer
    tx: Tx,

    /// The response consumer receives responses from the ingress manager
    res_c: FrameConsumer<'static, RES_CAPACITY>,
    /// The URC consumer receives URCs from the ingress manager
    urc_c: FrameConsumer<'static, URC_CAPACITY>,
    /// The command producer can send commands to the ingress manager
    com_p: ComProducer,

    state: ClientState,
    timer: CLK,
    config: Config,
}

impl<Tx, CLK, const TIMER_HZ: u32, const RES_CAPACITY: usize, const URC_CAPACITY: usize>
    Client<Tx, CLK, TIMER_HZ, RES_CAPACITY, URC_CAPACITY>
where
    Tx: serial::nb::Write<u8>,
    CLK: fugit_timer::Timer<TIMER_HZ>,
{
    pub fn new(
        tx: Tx,
        res_c: FrameConsumer<'static, RES_CAPACITY>,
        urc_c: FrameConsumer<'static, URC_CAPACITY>,
        com_p: ComProducer,
        mut timer: CLK,
        config: Config,
    ) -> Self {
        timer.start(config.cmd_cooldown.millis()).ok();

        Self {
            tx,
            res_c,
            urc_c,
            com_p,
            state: ClientState::Idle,
            config,
            timer,
        }
    }
}

impl<Tx, CLK, const TIMER_HZ: u32, const RES_CAPACITY: usize, const URC_CAPACITY: usize> AtatClient
    for Client<Tx, CLK, TIMER_HZ, RES_CAPACITY, URC_CAPACITY>
where
    Tx: serial::nb::Write<u8>,
    CLK: fugit_timer::Timer<TIMER_HZ>,
{
    fn send<A: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &A,
    ) -> nb::Result<A::Response, Error> {
        if let ClientState::Idle = self.state {
            // if A::FORCE_RECEIVE_STATE && self.com_p.enqueue(Command::ForceReceiveState).is_err() {
            //     // TODO: Consider how to act in this situation.
            //     error!("Failed to signal parser to force state transition to 'ReceivingResponse'!",);
            // }

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
                nb::block!(self.tx.write(c)).map_err(|_e| Error::Write)?;
            }
            nb::block!(self.tx.flush()).map_err(|_e| Error::Write)?;
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

            return cmd
                .parse(ResponseHeader::from_bytes(res_grant.as_ref()))
                .map_err(nb::Error::from)
                .and_then(|r| {
                    if let ClientState::AwaitingResponse = self.state {
                        self.timer.start(self.config.cmd_cooldown.millis()).ok();
                        self.state = ClientState::Idle;
                        Ok(r)
                    } else {
                        // FIXME: Is this correct?
                        error!("Is this correct?! WouldBlock");
                        Err(nb::Error::WouldBlock)
                    }
                })
                .map_err(|e| {
                    self.timer.start(self.config.cmd_cooldown.millis()).ok();
                    self.state = ClientState::Idle;
                    e
                });
        } else if let Mode::Timeout = self.config.mode {
            if self.timer.wait().is_ok() {
                self.state = ClientState::Idle;
                // Tell the parser to reset to initial state due to timeout
                if self.com_p.enqueue(Command::Reset).is_err() {
                    // TODO: Consider how to act in this situation.
                    error!("Failed to signal parser to clear buffer on timeout!");
                }
                return Err(nb::Error::Other(Error::Timeout));
            }
        }
        Err(nb::Error::WouldBlock)
    }

    fn get_mode(&self) -> Mode {
        self.config.mode
    }

    fn reset(&mut self) {
        if self.com_p.enqueue(Command::Reset).is_err() {
            // TODO: Consider how to act in this situation.
            error!("Failed to signal ingress manager to reset!");
        }
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
    use super::*;
    use crate::queues;
    use crate::{self as atat, InternalError};
    use crate::{
        atat_derive::{AtatCmd, AtatEnum, AtatResp, AtatUrc},
        Clock,
    };
    use bbqueue::framed::FrameProducer;
    use bbqueue::BBBuffer;
    use heapless::{spsc::Queue, String, Vec};
    use nb;

    const TEST_RX_BUF_LEN: usize = 256;
    const TEST_RES_CAPACITY: usize = 3 * TEST_RX_BUF_LEN;
    const TEST_URC_CAPACITY: usize = 3 * TEST_RX_BUF_LEN;
    const TIMER_HZ: u32 = 1000;

    struct CdMock<const TIMER_HZ: u32>;

    impl<const TIMER_HZ: u32> Clock<TIMER_HZ> for CdMock<TIMER_HZ> {
        type Error = core::convert::Infallible;

        /// Return current time `Instant`
        fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ> {
            fugit::TimerInstantU32::from_ticks(0)
        }

        /// Start countdown with a `duration`
        fn start(
            &mut self,
            _duration: fugit::TimerDurationU32<TIMER_HZ>,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        /// Stop timer
        fn cancel(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        /// Wait until countdown `duration` set with the `fn start` has expired
        fn wait(&mut self) -> nb::Result<(), Self::Error> {
            Ok(())
        }
    }

    #[derive(Debug)]
    pub enum SerialError {}

    impl embedded_hal::serial::Error for SerialError {
        fn kind(&self) -> serial::ErrorKind {
            serial::ErrorKind::Other
        }
    }

    struct TxMock {
        s: String<64>,
    }

    impl TxMock {
        fn new(s: String<64>) -> Self {
            TxMock { s }
        }
    }

    impl serial::nb::Write<u8> for TxMock {
        type Error = serial::ErrorKind;

        fn write(&mut self, c: u8) -> nb::Result<(), Self::Error> {
            self.s
                .push(c as char)
                .map_err(|_| nb::Error::Other(serial::ErrorKind::Other))
        }

        fn flush(&mut self) -> nb::Result<(), Self::Error> {
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
    #[at_cmd("+CUN", TestResponseStringMixed, timeout_ms = 180000)]
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

    #[derive(Clone, AtatResp)]
    pub struct MessageWaitingIndication {
        #[at_arg(position = 0)]
        pub status: u8,
        #[at_arg(position = 1)]
        pub code: u8,
    }

    #[derive(Clone, AtatUrc)]
    pub enum Urc {
        #[at_urc(b"+UMWI")]
        MessageWaitingIndication(MessageWaitingIndication),
    }

    macro_rules! setup {
        ($config:expr) => {{
            static mut RES_Q: BBBuffer<TEST_RES_CAPACITY> = BBBuffer::new();
            let (res_p, res_c) = unsafe { RES_Q.try_split_framed().unwrap() };

            static mut URC_Q: BBBuffer<TEST_URC_CAPACITY> = BBBuffer::new();
            let (urc_p, urc_c) = unsafe { URC_Q.try_split_framed().unwrap() };

            static mut COM_Q: queues::ComQueue = Queue::new();
            let (com_p, _com_c) = unsafe { COM_Q.split() };

            assert_eq!(com_p.capacity(), crate::queues::COM_CAPACITY);

            let tx_mock = TxMock::new(String::new());
            let client: Client<
                TxMock,
                CdMock<TIMER_HZ>,
                TIMER_HZ,
                TEST_RES_CAPACITY,
                TEST_URC_CAPACITY,
            > = Client::new(tx_mock, res_c, urc_c, com_p, CdMock, $config);
            (client, res_p, urc_p)
        }};
    }

    pub fn enqueue_res(
        producer: &mut FrameProducer<'static, TEST_RES_CAPACITY>,
        res: Result<&[u8], InternalError>,
    ) {
        let (header, bytes) = ResponseHeader::as_bytes(&res);

        if let Ok(mut grant) = producer.grant(bytes.len() + header.len()) {
            grant[0..header.len()].copy_from_slice(&header);
            grant[header.len()..header.len() + bytes.len()].copy_from_slice(bytes);
            grant.commit(bytes.len() + header.len());
        } else {
            // FIXME: Handle queue being full
            error!("Response queue full!");
        }
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
}
