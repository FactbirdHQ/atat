use heapless::{consts, spsc::Consumer, String};

use embedded_hal::serial;

use crate::error::{Error, NBResult, Result};
use crate::traits::{ATATCmd, ATATInterface, ATATUrc};
use crate::{Config, Mode};
use core::time::Duration;
use ticklock::timer::{Timer, TimerInstant};

type ResConsumer = Consumer<'static, Result<String<consts::U256>>, consts::U5, u8>;
type UrcConsumer = Consumer<'static, String<consts::U64>, consts::U10, u8>;

#[derive(Debug, PartialEq)]
enum ClientState {
    Idle,
    AwaitingResponse,
}

pub struct ATClient<Tx, T>
where
    Tx: serial::Write<u8>,
    T: Timer,
{
    tx: Tx,
    res_c: ResConsumer,
    urc_c: UrcConsumer,
    time_instant: Option<TimerInstant<T>>,
    last_comm_time: u32,
    state: ClientState,
    timer: Option<T>,
    config: Config,
}

impl<Tx, T> ATClient<Tx, T>
where
    Tx: serial::Write<u8>,
    T: Timer,
{
    pub fn new(
        tx: Tx,
        res_c: ResConsumer,
        urc_c: UrcConsumer,
        mut timer: T,
        config: Config,
    ) -> Self {
        Self {
            tx,
            res_c,
            urc_c,
            state: ClientState::Idle,
            last_comm_time: timer.get_current().into(),
            config,
            timer: Some(timer),
            time_instant: None,
        }
    }
}

impl<Tx, T> ATATInterface for ATClient<Tx, T>
where
    Tx: serial::Write<u8>,
    T: Timer,
{
    fn send<A: ATATCmd>(&mut self, cmd: &A) -> NBResult<A::Response> {
        if let ClientState::Idle = self.state {
            if let Some(ref mut timer) = self.timer {
                let delta: u32 = timer.get_current().into() - self.last_comm_time;
                if delta < self.config.cmd_cooldown {
                    timer.delay(Duration::from_millis(u64::from(
                        self.config.cmd_cooldown - delta,
                    )));
                }
                self.last_comm_time = timer.get_current().into();
            }
            for c in cmd.as_str().as_bytes() {
                block!(self.tx.write(*c)).ok();
            }
            block!(self.tx.flush()).ok();
            self.state = ClientState::AwaitingResponse;
        }

        let res = match self.config.mode {
            Mode::Blocking => Ok(block!(self.check_response(cmd))?),
            Mode::NonBlocking => self.check_response(cmd),
            Mode::Timeout => {
                if let Some(timer) = self.timer.take() {
                    self.time_instant = Some(timer.start());
                } else {
                    log::error!("Timer already started!!!");
                }
                Ok(block!(self.check_response(cmd))?)
            }
        };
        if let Some(ref mut timer) = self.timer {
            self.last_comm_time = timer.get_current().into();
        };

        res
    }

    fn check_urc<URC: ATATUrc>(&mut self) -> Option<URC::Resp> {
        if let Some(ref resp) = self.urc_c.dequeue() {
            if let Some(ref mut timer) = self.timer {
                self.last_comm_time = timer.get_current().into();
            };
            match URC::parse(resp) {
                Ok(r) => Some(r),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    fn check_response<A: ATATCmd>(&mut self, cmd: &A) -> NBResult<A::Response> {
        if let Some(result) = self.res_c.dequeue() {
            return match result {
                Ok(ref resp) => {
                    if let ClientState::AwaitingResponse = self.state {
                        if let Some(ti) = self.time_instant.take() {
                            self.timer = Some(ti.stop());
                        };
                        self.state = ClientState::Idle;
                        Ok(cmd.parse(resp).map_err(nb::Error::Other)?)
                    } else {
                        Err(nb::Error::WouldBlock)
                    }
                }
                Err(e) => Err(nb::Error::Other(e)),
            };
        } else if let Mode::Timeout = self.config.mode {
            let timed_out = if let Some(timer) = self.time_instant.as_mut() {
                timer
                    .wait(Duration::from_millis(cmd.max_timeout_ms().into()))
                    .is_ok()
            } else {
                log::error!("TimeInstant already consumed!!");
                true
            };

            if timed_out {
                if let Some(ti) = self.time_instant.take() {
                    self.timer = Some(ti.stop());
                }
                self.state = ClientState::Idle;
                return Err(nb::Error::Other(Error::Timeout));
            }
        }
        Err(nb::Error::WouldBlock)
    }
}
#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod test {
    use super::*;
    use crate as atat;
    use crate::atat_derive::{ATATCmd, ATATResp, ATATUrc};
    use heapless::{consts, spsc::Queue, String, Vec};
    use nb;
    use serde;
    use serde_repr::{Deserialize_repr, Serialize_repr};
    use ticklock::timer::{Timer, TimerInstant};

    struct CdMock {
        time: u32,
    }

    impl Timer for CdMock {
        type U = u32;

        // fn start<T>(&mut self, count: T)
        // where
        //     T: Into<Self::Time>,
        // {
        //     self.time = count.into();
        // }
        // fn wait(&mut self) -> nb::Result<(), Void> {
        //     Ok(())
        // }

        fn delay(&mut self, _d: Duration) {}

        fn get_current(&mut self) -> Self::U {
            self.time
        }

        fn limit_value(&self) -> Self::U {
            0
        }

        fn has_wrapped(&mut self) -> bool {
            false
        }

        fn stop(self) -> Self {
            self
        }

        fn start(self) -> TimerInstant<Self> {
            TimerInstant::now(CdMock { time: 0 })
        }

        fn tick(&mut self) -> Duration {
            Duration::from_millis(1)
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

    #[derive(Clone, ATATCmd)]
    #[at_cmd("+CFUN", NoResonse, timeout_ms = 180000)]
    pub struct SetModuleFunctionality {
        #[at_arg(position = 0)]
        pub fun: Functionality,
        #[at_arg(position = 1)]
        pub rst: Option<ResetMode>,
    }

    #[derive(Clone, ATATCmd)]
    #[at_cmd("+FUN", NoResonse, timeout_ms = 180000)]
    pub struct Test2Cmd {
        #[at_arg(position = 1)]
        pub fun: Functionality,
        #[at_arg(position = 0)]
        pub rst: Option<ResetMode>,
    }
    #[derive(Clone, ATATCmd)]
    #[at_cmd("+CUN", TestResponseVec, timeout_ms = 180000)]
    pub struct TestRespVecCmd {
        #[at_arg(position = 0)]
        pub fun: Functionality,
        #[at_arg(position = 1)]
        pub rst: Option<ResetMode>,
    }
    #[derive(Clone, ATATCmd)]
    #[at_cmd("+CUN", TestResponseString, timeout_ms = 180000)]
    pub struct TestRespStringCmd {
        #[at_arg(position = 0)]
        pub fun: Functionality,
        #[at_arg(position = 1)]
        pub rst: Option<ResetMode>,
    }
    #[derive(Clone, ATATCmd)]
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
    #[derive(Clone, ATATResp, PartialEq, Debug)]
    pub struct NoResonse;
    #[derive(Clone, ATATResp, PartialEq, Debug)]
    pub struct TestResponseVec {
        #[at_arg(position = 0)]
        pub socket: u8,
        #[at_arg(position = 1)]
        pub length: usize,
        #[at_arg(position = 2)]
        pub data: Vec<u8, consts::U256>,
    }

    #[derive(Clone, ATATResp, PartialEq, Debug)]
    pub struct TestResponseString {
        #[at_arg(position = 0)]
        pub socket: u8,
        #[at_arg(position = 1)]
        pub length: usize,
        #[at_arg(position = 2)]
        pub data: String<consts::U64>,
    }

    #[derive(Clone, ATATResp, PartialEq, Debug)]
    pub struct TestResponseStringMixed {
        #[at_arg(position = 1)]
        pub socket: u8,
        #[at_arg(position = 2)]
        pub length: usize,
        #[at_arg(position = 0)]
        pub data: String<consts::U64>,
    }

    #[derive(Clone, ATATResp)]
    pub struct MessageWaitingIndication {
        #[at_arg(position = 0)]
        pub status: u8,
        #[at_arg(position = 1)]
        pub code: u8,
    }

    #[derive(Clone, ATATUrc)]
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

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: ATClient<TxMock, CdMock> =
            ATClient::new(tx_mock, c, urc_c, timer, Config::new(Mode::Blocking));

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
            String::<consts::U32>::from("AT+CFUN=4,0\r"),
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
            String::<consts::U32>::from("AT+CFUN=4,0\rAT+FUN=1,6\r"),
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

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: ATClient<TxMock, CdMock> =
            ATClient::new(tx_mock, c, urc_c, timer, Config::new(Mode::Timeout));

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

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: ATClient<TxMock, CdMock> =
            ATClient::new(tx_mock, c, urc_c, timer, Config::new(Mode::Blocking));

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
        assert_eq!(client.tx.s, String::<consts::U32>::from("AT+CFUN=4,0\r"));
    }

    #[test]
    fn non_blocking() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (mut p, c) = unsafe { REQ_Q.split() };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (_urc_p, urc_c) = unsafe { URC_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: ATClient<TxMock, CdMock> =
            ATClient::new(tx_mock, c, urc_c, timer, Config::new(Mode::NonBlocking));

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

    //Testing unsupported frature in form of vec deserialization
    #[test]
    #[ignore]
    fn response_vec() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (mut p, c) = unsafe { REQ_Q.split() };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (_urc_p, urc_c) = unsafe { URC_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: ATClient<TxMock, CdMock> =
            ATClient::new(tx_mock, c, urc_c, timer, Config::new(Mode::Blocking));

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
    //Test response containing string
    #[test]
    fn response_string() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (mut p, c) = unsafe { REQ_Q.split() };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (_urc_p, urc_c) = unsafe { URC_Q.split() };

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: ATClient<TxMock, CdMock> =
            ATClient::new(tx_mock, c, urc_c, timer, Config::new(Mode::Blocking));

        //String last
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

        //Mixed order for string
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

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: ATClient<TxMock, CdMock> =
            ATClient::new(tx_mock, c, urc_c, timer, Config::new(Mode::NonBlocking));

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

        let timer = CdMock { time: 0 };

        let tx_mock = TxMock::new(String::new());
        let mut client: ATClient<TxMock, CdMock> =
            ATClient::new(tx_mock, c, urc_c, timer, Config::new(Mode::Blocking));

        //String last
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
