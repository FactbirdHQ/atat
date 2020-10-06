use embedded_hal::{serial, timer::CountDown};

use crate::atat_log;
use crate::error::Error;
use crate::queues::{ComItem, ComProducer, ResConsumer, ResItem, UrcConsumer, UrcItem};
use crate::traits::{AtatClient, AtatCmd, AtatUrc};
use crate::{Command, Config, Mode};
use heapless::{consts, ArrayLength};

#[derive(Debug, PartialEq)]
enum ClientState {
    Idle,
    AwaitingResponse,
}

/// Client responsible for handling send, receive and timeout from the
/// userfacing side. The client is decoupled from the ingress-manager through
/// some spsc queue consumers, where any received responses can be dequeued. The
/// Client also has an spsc producer, to allow signaling commands like
/// `clearBuffer` to the ingress-manager.
pub struct Client<
    Tx,
    T,
    BufLen = consts::U256,
    ComCapacity = consts::U3,
    ResCapacity = consts::U5,
    UrcCapacity = consts::U10,
> where
    Tx: serial::Write<u8>,
    T: CountDown,
    BufLen: ArrayLength<u8>,
    ComCapacity: ArrayLength<ComItem>,
    ResCapacity: ArrayLength<ResItem<BufLen>>,
    UrcCapacity: ArrayLength<UrcItem<BufLen>>,
{
    /// Serial writer
    tx: Tx,

    /// The response consumer receives responses from the ingress manager
    res_c: ResConsumer<BufLen, ResCapacity>,
    /// The URC consumer receives URCs from the ingress manager
    urc_c: UrcConsumer<BufLen, UrcCapacity>,
    /// The command producer can send commands to the ingress manager
    com_p: ComProducer<ComCapacity>,

    state: ClientState,
    timer: T,
    config: Config,
}

impl<Tx, T, BufLen, ComCapacity, ResCapacity, UrcCapacity>
    Client<Tx, T, BufLen, ComCapacity, ResCapacity, UrcCapacity>
where
    Tx: serial::Write<u8>,
    T: CountDown,
    T::Time: From<u32>,
    BufLen: ArrayLength<u8>,
    ComCapacity: ArrayLength<ComItem>,
    ResCapacity: ArrayLength<ResItem<BufLen>>,
    UrcCapacity: ArrayLength<UrcItem<BufLen>>,
{
    pub fn new(
        tx: Tx,
        res_c: ResConsumer<BufLen, ResCapacity>,
        urc_c: UrcConsumer<BufLen, UrcCapacity>,
        com_p: ComProducer<ComCapacity>,
        timer: T,
        config: Config,
    ) -> Self {
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

impl<Tx, T, BufLen, ComCapacity, ResCapacity, UrcCapacity> AtatClient
    for Client<Tx, T, BufLen, ComCapacity, ResCapacity, UrcCapacity>
where
    Tx: serial::Write<u8>,
    T: CountDown,
    T::Time: From<u32>,
    BufLen: ArrayLength<u8>,
    ComCapacity: ArrayLength<ComItem>,
    ResCapacity: ArrayLength<ResItem<BufLen>>,
    UrcCapacity: ArrayLength<UrcItem<BufLen>>,
{
    fn send<A: AtatCmd>(&mut self, cmd: &A) -> nb::Result<A::Response, Error> {
        if let ClientState::Idle = self.state {
            if cmd.force_receive_state()
                && self
                    .com_p
                    .enqueue(Command::ForceState(
                        crate::ingress_manager::State::ReceivingResponse,
                    ))
                    .is_err()
            {
                // TODO: Consider how to act in this situation.
                atat_log!(
                    error,
                    "Failed to signal parser to force state transition to 'ReceivingResponse'!"
                );
            }

            // compare the time of the last response or URC and ensure at least
            // `self.config.cmd_cooldown` ms have passed before sending a new
            // command
            nb::block!(self.timer.wait()).ok();
            let cmd_buf = cmd.as_bytes();

            #[allow(clippy::single_match)]
            match core::str::from_utf8(&cmd_buf) {
                Ok(_s) => {
                    #[cfg(not(feature = "log-logging"))]
                    atat_log!(debug, "Sending command: {:str}", _s);
                    #[cfg(feature = "log-logging")]
                    atat_log!(debug, "Sending command: {:?}", _s);
                }
                Err(_) => atat_log!(
                    debug,
                    "Sending command: {:?}",
                    core::convert::AsRef::<[u8]>::as_ref(&cmd_buf)
                ),
            };

            for c in cmd_buf {
                nb::block!(self.tx.write(c)).map_err(|_e| Error::Write)?;
            }
            nb::block!(self.tx.flush()).map_err(|_e| Error::Write)?;
            self.state = ClientState::AwaitingResponse;
        }

        match self.config.mode {
            Mode::Blocking => Ok(nb::block!(self.check_response(cmd))?),
            Mode::NonBlocking => self.check_response(cmd),
            Mode::Timeout => {
                self.timer.start(cmd.max_timeout_ms());
                Ok(nb::block!(self.check_response(cmd))?)
            }
        }
    }

    fn check_urc<URC: AtatUrc>(&mut self) -> Option<URC::Response> {
        if !self.urc_c.ready() {
            return None;
        }

        self.timer.start(self.config.cmd_cooldown);
        URC::parse(unsafe { &self.urc_c.dequeue_unchecked() }).ok()
    }

    fn check_response<A: AtatCmd>(&mut self, cmd: &A) -> nb::Result<A::Response, Error> {
        if let Some(result) = self.res_c.dequeue() {
            return match result {
                Ok(ref resp) => {
                    if let ClientState::AwaitingResponse = self.state {
                        self.timer.start(self.config.cmd_cooldown);
                        self.state = ClientState::Idle;
                        Ok(cmd.parse(resp).map_err(nb::Error::Other)?)
                    } else {
                        Err(nb::Error::WouldBlock)
                    }
                }
                Err(e) => Err(nb::Error::Other(e)),
            };
        } else if let Mode::Timeout = self.config.mode {
            if self.timer.wait().is_ok() {
                self.state = ClientState::Idle;
                // Tell the parser to clear the buffer due to timeout
                if self.com_p.enqueue(Command::ClearBuffer).is_err() {
                    // TODO: Consider how to act in this situation.
                    atat_log!(error, "Failed to signal parser to clear buffer on timeout!");
                }
                return Err(nb::Error::Other(Error::Timeout));
            }
        }
        Err(nb::Error::WouldBlock)
    }

    fn get_mode(&self) -> Mode {
        self.config.mode
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate as atat;
    use crate::atat_derive::{AtatCmd, AtatEnum, AtatResp, AtatUrc};
    use crate::queues;
    use heapless::{consts, spsc::Queue, String, Vec};
    use nb;
    use serde;
    use void::Void;

    struct CdMock {
        time: u32,
    }

    impl CountDown for CdMock {
        type Time = u32;
        fn start<T>(&mut self, count: T)
        where
            T: Into<Self::Time>,
        {
            self.time = count.into();
        }
        fn wait(&mut self) -> nb::Result<(), Void> {
            Ok(())
        }
    }

    struct TxMock {
        s: String<consts::U64>,
    }

    impl TxMock {
        fn new(s: String<consts::U64>) -> Self {
            TxMock { s }
        }
    }

    impl serial::Write<u8> for TxMock {
        type Error = ();

        fn write(&mut self, c: u8) -> nb::Result<(), Self::Error> {
            self.s.push(c as char).map_err(nb::Error::Other)
        }

        fn flush(&mut self) -> nb::Result<(), Self::Error> {
            Ok(())
        }
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
    #[at_cmd("+CUN", TestResponseVec, timeout_ms = 180000)]
    pub struct TestRespVecCmd {
        #[at_arg(position = 0)]
        pub fun: Functionality,
        #[at_arg(position = 1)]
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
    pub struct TestResponseVec {
        #[at_arg(position = 0)]
        pub socket: u8,
        #[at_arg(position = 1)]
        pub length: usize,
        #[at_arg(position = 2)]
        pub data: Vec<u8, TestRxBufLen>,
    }

    #[derive(Clone, AtatResp, PartialEq, Debug)]
    pub struct TestResponseString {
        #[at_arg(position = 0)]
        pub socket: u8,
        #[at_arg(position = 1)]
        pub length: usize,
        #[at_arg(position = 2)]
        pub data: String<consts::U64>,
    }

    #[derive(Clone, AtatResp, PartialEq, Debug)]
    pub struct TestResponseStringMixed {
        #[at_arg(position = 1)]
        pub socket: u8,
        #[at_arg(position = 2)]
        pub length: usize,
        #[at_arg(position = 0)]
        pub data: String<consts::U64>,
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

    type TestRxBufLen = consts::U256;
    type TestComCapacity = consts::U3;
    type TestResCapacity = consts::U5;
    type TestUrcCapacity = consts::U10;

    macro_rules! setup {
        ($config:expr) => {{
            static mut RES_Q: queues::ResQueue<TestRxBufLen, TestResCapacity> =
                Queue(heapless::i::Queue::u8());
            let (res_p, res_c) = unsafe { RES_Q.split() };
            static mut URC_Q: queues::UrcQueue<TestRxBufLen, TestUrcCapacity> =
                Queue(heapless::i::Queue::u8());
            let (urc_p, urc_c) = unsafe { URC_Q.split() };
            static mut COM_Q: queues::ComQueue<TestComCapacity> = Queue(heapless::i::Queue::u8());
            let (com_p, _com_c) = unsafe { COM_Q.split() };

            let timer = CdMock { time: 0 };

            let tx_mock = TxMock::new(String::new());
            let client: Client<
                TxMock,
                CdMock,
                TestRxBufLen,
                TestComCapacity,
                TestResCapacity,
                TestUrcCapacity,
            > = Client::new(tx_mock, res_c, urc_c, com_p, timer, $config);
            (client, res_p, urc_p)
        }};
    }

    #[test]
    fn string_sent() {
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking));

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        p.enqueue(Ok(Vec::<u8, TestRxBufLen>::new())).unwrap();

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.send(&cmd), Ok(NoResponse));
        assert_eq!(client.state, ClientState::Idle);

        assert_eq!(
            client.tx.s,
            String::<consts::U32>::from("AT+CFUN=4,0\r\n"),
            "Wrong encoding of string"
        );

        p.enqueue(Ok(Vec::<u8, TestRxBufLen>::new())).unwrap();

        let cmd = Test2Cmd {
            fun: Functionality::DM,
            rst: Some(ResetMode::Reset),
        };
        assert_eq!(client.send(&cmd), Ok(NoResponse));

        assert_eq!(
            client.tx.s,
            String::<consts::U32>::from("AT+CFUN=4,0\r\nAT+FUN=1,6\r\n"),
            "Reverse order string did not match"
        );
    }

    #[test]
    #[ignore]
    fn countdown() {
        let (mut client, _, _) = setup!(Config::new(Mode::Timeout));

        assert_eq!(client.state, ClientState::Idle);

        let cmd = Test2Cmd {
            fun: Functionality::DM,
            rst: Some(ResetMode::Reset),
        };
        assert_eq!(client.send(&cmd), Err(nb::Error::Other(Error::Timeout)));

        // TODO: Test countdown is recived corretly
        match client.config.mode {
            Mode::Timeout => {} // assert_eq!(cd_mock.time, 180000),
            _ => panic!("Wrong AT mode"),
        }
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn blocking() {
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking));

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        p.enqueue(Ok(Vec::<u8, TestRxBufLen>::new())).unwrap();

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.send(&cmd), Ok(NoResponse));
        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.tx.s, String::<consts::U32>::from("AT+CFUN=4,0\r\n"));
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

        p.enqueue(Ok(Vec::<u8, TestRxBufLen>::new())).unwrap();

        assert_eq!(client.state, ClientState::AwaitingResponse);

        assert_eq!(client.check_response(&cmd), Ok(NoResponse));
        assert_eq!(client.state, ClientState::Idle);
    }

    // Testing unsupported feature in form of vec deserialization
    #[test]
    #[ignore]
    fn response_vec() {
        let (mut client, mut p, _) = setup!(Config::new(Mode::Blocking));

        let cmd = TestRespVecCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let response =
            Vec::<u8, TestRxBufLen>::from_slice(b"+CUN: 22,16,\"0123456789012345\"").unwrap();
        p.enqueue(Ok(response)).unwrap();

        let res_vec: Vec<u8, TestRxBufLen> =
            "0123456789012345".as_bytes().iter().cloned().collect();

        assert_eq!(client.state, ClientState::Idle);

        assert_eq!(
            client.send(&cmd),
            Ok(TestResponseVec {
                socket: 22,
                length: 16,
                data: res_vec
            })
        );
        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.tx.s, String::<consts::U32>::from("AT+CFUN=4,0\r\n"));
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

        let response =
            Vec::<u8, TestRxBufLen>::from_slice(b"+CUN: 22,16,\"0123456789012345\"").unwrap();
        p.enqueue(Ok(response)).unwrap();

        assert_eq!(client.state, ClientState::Idle);

        assert_eq!(
            client.send(&cmd),
            Ok(TestResponseString {
                socket: 22,
                length: 16,
                data: String::<consts::U64>::from("0123456789012345")
            })
        );
        assert_eq!(client.state, ClientState::Idle);

        // Mixed order for string
        let cmd = TestRespStringMixCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let response =
            Vec::<u8, TestRxBufLen>::from_slice(b"+CUN: \"0123456789012345\",22,16").unwrap();
        p.enqueue(Ok(response)).unwrap();

        assert_eq!(
            client.send(&cmd),
            Ok(TestResponseStringMixed {
                socket: 22,
                length: 16,
                data: String::<consts::U64>::from("0123456789012345")
            })
        );
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn urc() {
        let (mut client, _, mut urc_p) = setup!(Config::new(Mode::NonBlocking));

        let response = Vec::<u8, TestRxBufLen>::from_slice(b"+UMWI: 0, 1").unwrap();
        urc_p.enqueue(response).unwrap();

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

        let response = Vec::<u8, TestRxBufLen>::from_slice(b"+CUN: 22,16,22").unwrap();
        p.enqueue(Ok(response)).unwrap();

        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.send(&cmd), Err(nb::Error::Other(Error::ParseString)));
        assert_eq!(client.state, ClientState::Idle);
    }
}
