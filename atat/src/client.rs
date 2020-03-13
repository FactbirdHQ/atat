use heapless::{
    consts,
    spsc::{Consumer, Producer},
    String,
};

use embedded_hal::{serial, timer::CountDown};

use crate::error::{Error, NBResult, Result};
use crate::traits::{AtatCmd, AtatClient, AtatUrc};
use crate::{Command, Config, Mode};

type ResConsumer = Consumer<'static, Result<String<consts::U256>>, consts::U5, u8>;
type UrcConsumer = Consumer<'static, String<consts::U64>, consts::U10, u8>;
type ComProducer = Producer<'static, Command, consts::U3, u8>;

#[derive(Debug, PartialEq)]
enum ClientState {
    Idle,
    AwaitingResponse,
}

/// Client responsible for handling send, receive and timeout from the
/// userfacing side. The client is decoupled from the ingress-manager through
/// some spsc queue consumers, where any received responses can be dequeued. The
/// Client also has an spsc producer, to allow signaling commands like
/// 'clearBuffer' to the ingress-manager.
pub struct Client<Tx, T>
where
    Tx: serial::Write<u8>,
    T: CountDown,
{
    tx: Tx,
    res_c: ResConsumer,
    urc_c: UrcConsumer,
    com_p: ComProducer,
    state: ClientState,
    timer: T,
    config: Config,
}

impl<Tx, T> Client<Tx, T>
where
    Tx: serial::Write<u8>,
    T: CountDown,
    T::Time: From<u32>,
{
    pub fn new(
        tx: Tx,
        res_c: ResConsumer,
        urc_c: UrcConsumer,
        com_p: ComProducer,
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

impl<Tx, T> AtatClient for Client<Tx, T>
where
    Tx: serial::Write<u8>,
    T: CountDown,
    T::Time: From<u32>,
{
    fn send<A: AtatCmd>(&mut self, cmd: &A) -> NBResult<A::Response> {
        if let ClientState::Idle = self.state {
            // compare the time of the last response or URC and ensure at least
            // `self.config.cmd_cooldown` ms have passed before sending a new
            // command
            block!(self.timer.wait()).ok();
            for c in cmd.as_string().as_bytes() {
                block!(self.tx.write(*c)).ok();
            }
            block!(self.tx.flush()).ok();
            self.state = ClientState::AwaitingResponse;
        }

        match self.config.mode {
            Mode::Blocking => Ok(block!(self.check_response(cmd))?),
            Mode::NonBlocking => self.check_response(cmd),
            Mode::Timeout => {
                self.timer.start(cmd.max_timeout_ms());
                Ok(block!(self.check_response(cmd))?)
            }
        }
    }

    fn check_urc<URC: AtatUrc>(&mut self) -> Option<URC::Resp> {
        if let Some(ref resp) = self.urc_c.dequeue() {
            self.timer.start(self.config.cmd_cooldown);
            match URC::parse(resp) {
                Ok(r) => Some(r),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    fn check_response<A: AtatCmd>(&mut self, cmd: &A) -> NBResult<A::Response> {
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
                    // log::error!("Failed to signal parser to clear buffer on timeout!\r");
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
#[cfg_attr(tarpaulin, skip)]
mod test {
    use super::*;
    use crate as atat;
    use crate::atat_derive::{AtatCmd, AtatResp, AtatUrc};
    use heapless::{consts, spsc::Queue, String, Vec};
    use nb;
    use serde;
    use serde_repr::{Deserialize_repr, Serialize_repr};
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
            //TODO: this just feels wrong..
            match self.s.push(c as char) {
                Ok(_) => Ok(()),
                Err(_) => Err(nb::Error::Other(())),
            }
        }

        fn flush(&mut self) -> nb::Result<(), Self::Error> {
            Ok(())
        }
    }

    #[derive(Clone, AtatCmd)]
    #[at_cmd("+CFUN", NoResonse, timeout_ms = 180000)]
    pub struct SetModuleFunctionality {
        #[at_arg(position = 0)]
        pub fun: Functionality,
        #[at_arg(position = 1)]
        pub rst: Option<ResetMode>,
    }

    #[derive(Clone, AtatCmd)]
    #[at_cmd("+FUN", NoResonse, timeout_ms = 180000)]
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

    #[derive(Clone, PartialEq, Serialize_repr, Deserialize_repr)]
    #[repr(u8)]
    pub enum Functionality {
        Min = 0,
        Full = 1,
        APM = 4,
        DM = 6,
    }
    #[derive(Clone, PartialEq, Serialize_repr, Deserialize_repr)]
    #[repr(u8)]
    pub enum ResetMode {
        DontReset = 0,
        Reset = 1,
    }
    #[derive(Clone, AtatResp, PartialEq, Debug)]
    pub struct NoResonse;
    #[derive(Clone, AtatResp, PartialEq, Debug)]
    pub struct TestResponseVec {
        #[at_arg(position = 0)]
        pub socket: u8,
        #[at_arg(position = 1)]
        pub length: usize,
        #[at_arg(position = 2)]
        pub data: Vec<u8, consts::U256>,
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
        #[at_urc("+UMWI")]
        MessageWaitingIndication(MessageWaitingIndication),
    }

    #[test]
    fn string_sent() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (mut p, c) = unsafe { REQ_Q.split() };
        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (_urc_p, urc_c) = unsafe { URC_Q.split() };
        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (com_p, _com_c) = unsafe { COM_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: Client<TxMock, CdMock> =
            Client::new(tx_mock, c, urc_c, com_p, timer, Config::new(Mode::Blocking));

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let resp: Result<String<consts::U256>> = Ok(String::<consts::U256>::from(""));
        p.enqueue(resp).unwrap();

        assert_eq!(client.state, ClientState::Idle);

        match client.send(&cmd) {
            Ok(response) => {
                assert_eq!(response, NoResonse);
            }
            _ => panic!("Panic send error in test."),
        }
        assert_eq!(client.state, ClientState::Idle);

        assert_eq!(
            client.tx.s,
            String::<consts::U32>::from("AT+CFUN=4,0\r\n"),
            "Wrong encoding of string"
        );

        let resp: Result<String<consts::U256>> = Ok(String::<consts::U256>::from(""));
        p.enqueue(resp).unwrap();

        let cmd = Test2Cmd {
            fun: Functionality::DM,
            rst: Some(ResetMode::Reset),
        };
        match client.send(&cmd) {
            Ok(response) => {
                assert_eq!(response, NoResonse);
            }
            _ => panic!("Panic send error in test."),
        }

        assert_eq!(
            client.tx.s,
            String::<consts::U32>::from("AT+CFUN=4,0\r\nAT+FUN=1,6\r\n"),
            "Reverse order string did not match"
        );
    }

    #[test]
    #[ignore]
    fn countdown() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let c = unsafe { REQ_Q.split().1 };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (_urc_p, urc_c) = unsafe { URC_Q.split() };
        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (com_p, _com_c) = unsafe { COM_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: Client<TxMock, CdMock> =
            Client::new(tx_mock, c, urc_c, com_p, timer, Config::new(Mode::Timeout));

        assert_eq!(client.state, ClientState::Idle);

        let cmd = Test2Cmd {
            fun: Functionality::DM,
            rst: Some(ResetMode::Reset),
        };
        match client.send(&cmd) {
            Err(nb::Error::Other(error)) => assert_eq!(error, Error::Timeout),
            _ => panic!("Panic send error in test."),
        }
        //Todo: Test countdown is recived corretly
        match client.config.mode {
            Mode::Timeout => {} // assert_eq!(cd_mock.time, 180000),
            _ => panic!("Wrong AT mode"),
        }
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn blocking() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (mut p, c) = unsafe { REQ_Q.split() };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (_urc_p, urc_c) = unsafe { URC_Q.split() };

        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (com_p, _com_c) = unsafe { COM_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: Client<TxMock, CdMock> =
            Client::new(tx_mock, c, urc_c, com_p, timer, Config::new(Mode::Blocking));

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let resp: Result<String<consts::U256>> = Ok(String::<consts::U256>::from(""));
        p.enqueue(resp).unwrap();

        assert_eq!(client.state, ClientState::Idle);

        match client.send(&cmd) {
            Ok(response) => {
                assert_eq!(response, NoResonse);
            }
            _ => panic!("Panic send error in test."),
        }
        assert_eq!(client.state, ClientState::Idle);
        assert_eq!(client.tx.s, String::<consts::U32>::from("AT+CFUN=4,0\r\n"));
    }

    #[test]
    fn non_blocking() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (mut p, c) = unsafe { REQ_Q.split() };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (_urc_p, urc_c) = unsafe { URC_Q.split() };

        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (com_p, _com_c) = unsafe { COM_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: Client<TxMock, CdMock> = Client::new(
            tx_mock,
            c,
            urc_c,
            com_p,
            timer,
            Config::new(Mode::NonBlocking),
        );

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        assert_eq!(client.state, ClientState::Idle);

        match client.send(&cmd) {
            Err(error) => assert_eq!(error, nb::Error::WouldBlock),
            _ => panic!("Panic send error in test"),
        }

        assert_eq!(client.state, ClientState::AwaitingResponse);

        match client.check_response(&cmd) {
            Err(error) => assert_eq!(error, nb::Error::WouldBlock),
            _ => panic!("Send error in test"),
        }

        let resp: Result<String<consts::U256>> = Ok(String::<consts::U256>::from(""));
        p.enqueue(resp).unwrap();

        assert_eq!(client.state, ClientState::AwaitingResponse);

        match client.check_response(&cmd) {
            Ok(response) => {
                assert_eq!(response, NoResonse);
            }
            _ => panic!("Panic send error in test."),
        }
        assert_eq!(client.state, ClientState::Idle);
    }

    // Testing unsupported frature in form of vec deserialization
    #[test]
    #[ignore]
    fn response_vec() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (mut p, c) = unsafe { REQ_Q.split() };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (_urc_p, urc_c) = unsafe { URC_Q.split() };

        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (com_p, _com_c) = unsafe { COM_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: Client<TxMock, CdMock> =
            Client::new(tx_mock, c, urc_c, com_p, timer, Config::new(Mode::Blocking));

        let cmd = TestRespVecCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let resp: Result<String<consts::U256>> = Ok(String::<consts::U256>::from(
            "+CUN: 22,16,\"0123456789012345\"",
        ));
        p.enqueue(resp).unwrap();

        let res_vec: Vec<u8, consts::U256> =
            "0123456789012345".as_bytes().iter().cloned().collect();

        assert_eq!(client.state, ClientState::Idle);

        match client.send(&cmd) {
            Ok(response) => {
                assert_eq!(
                    response,
                    TestResponseVec {
                        socket: 22,
                        length: 16,
                        data: res_vec
                    }
                );
            }
            Err(error) => panic!("Panic send error in test: {:?}", error),
        }
        assert_eq!(client.state, ClientState::Idle);

        assert_eq!(client.tx.s, String::<consts::U32>::from("AT+CFUN=4,0\r\n"));
    }
    // Test response containing string
    #[test]
    fn response_string() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (mut p, c) = unsafe { REQ_Q.split() };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (_urc_p, urc_c) = unsafe { URC_Q.split() };

        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (com_p, _com_c) = unsafe { COM_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: Client<TxMock, CdMock> =
            Client::new(tx_mock, c, urc_c, com_p, timer, Config::new(Mode::Blocking));

        // String last
        let cmd = TestRespStringCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let resp: Result<String<consts::U256>> = Ok(String::<consts::U256>::from(
            "+CUN: 22,16,\"0123456789012345\"",
        ));
        p.enqueue(resp).unwrap();

        assert_eq!(client.state, ClientState::Idle);

        match client.send(&cmd) {
            Ok(response) => {
                assert_eq!(
                    response,
                    TestResponseString {
                        socket: 22,
                        length: 16,
                        data: String::<consts::U64>::from("0123456789012345")
                    }
                );
            }
            Err(error) => panic!("Panic send error in test: {:?}", error),
        }
        assert_eq!(client.state, ClientState::Idle);

        // Mixed order for string
        let cmd = TestRespStringMixCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let resp: Result<String<consts::U256>> = Ok(String::<consts::U256>::from(
            "+CUN: \"0123456789012345\",22,16",
        ));
        p.enqueue(resp).unwrap();

        match client.send(&cmd) {
            Ok(response) => {
                assert_eq!(
                    response,
                    TestResponseStringMixed {
                        socket: 22,
                        length: 16,
                        data: String::<consts::U64>::from("0123456789012345")
                    }
                );
            }
            Err(error) => panic!("Panic send error in test: {:?}", error),
        }
        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn urc() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (_p, c) = unsafe { REQ_Q.split() };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (mut urc_p, urc_c) = unsafe { URC_Q.split() };

        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (com_p, _com_c) = unsafe { COM_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: Client<TxMock, CdMock> = Client::new(
            tx_mock,
            c,
            urc_c,
            com_p,
            timer,
            Config::new(Mode::NonBlocking),
        );

        urc_p
            .enqueue(String::<consts::U64>::from("+UMWI: 0, 1"))
            .unwrap();

        assert_eq!(client.state, ClientState::Idle);

        match client.check_urc::<Urc>() {
            Some(_) => {}
            _ => panic!("Send error in test"),
        }

        assert_eq!(client.state, ClientState::Idle);
    }

    #[test]
    fn invalid_response() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (mut p, c) = unsafe { REQ_Q.split() };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (_urc_p, urc_c) = unsafe { URC_Q.split() };

        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (com_p, _com_c) = unsafe { COM_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: Client<TxMock, CdMock> =
            Client::new(tx_mock, c, urc_c, com_p, timer, Config::new(Mode::Blocking));

        // String last
        let cmd = TestRespStringCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let resp: Result<String<consts::U256>> = Ok(String::<consts::U256>::from("+CUN: 22,16,22"));
        p.enqueue(resp).unwrap();

        assert_eq!(client.state, ClientState::Idle);

        match client.send(&cmd) {
            Err(error) => assert_eq!(error, nb::Error::Other(Error::InvalidResponse)),
            _ => panic!("Panic send error in test"),
        }
        assert_eq!(client.state, ClientState::Idle);
    }
}
