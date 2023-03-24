use bbqueue::framed::FrameConsumer;
use embassy_time::Duration;
use embedded_io::blocking::Write;

use super::timer::Timer;
use super::AtatClient;
use crate::error::{Error, Response};
use crate::frame::Frame;
use crate::helpers::LossyStr;
use crate::AtatCmd;
use crate::{AtatUrc, Config};

/// Client responsible for handling send, receive and timeout from the
/// userfacing side. The client is decoupled from the ingress-manager through
/// some spsc queue consumers, where any received responses can be dequeued. The
/// Client also has an spsc producer, to allow signaling commands like
/// `reset` to the ingress-manager.
pub struct Client<'a, W, const RES_CAPACITY: usize, const URC_CAPACITY: usize>
where
    W: Write,
{
    writer: W,

    res_reader: FrameConsumer<'a, RES_CAPACITY>,
    urc_reader: FrameConsumer<'a, URC_CAPACITY>,

    cooldown_timer: Option<Timer>,
    config: Config,
}

impl<'a, W, const RES_CAPACITY: usize, const URC_CAPACITY: usize>
    Client<'a, W, RES_CAPACITY, URC_CAPACITY>
where
    W: Write,
{
    pub(crate) fn new(
        writer: W,
        res_reader: FrameConsumer<'a, RES_CAPACITY>,
        urc_reader: FrameConsumer<'a, URC_CAPACITY>,
        config: Config,
    ) -> Self {
        Self {
            writer,
            res_reader,
            urc_reader,
            cooldown_timer: None,
            config,
        }
    }

    fn start_cooldown_timer(&mut self) {
        self.cooldown_timer = Some(Timer::after(self.config.cmd_cooldown));
    }

    fn wait_cooldown_timer(&mut self) {
        if let Some(cooldown) = self.cooldown_timer.take() {
            cooldown.wait();
        }
    }
}

impl<W, const RES_CAPACITY: usize, const URC_CAPACITY: usize> AtatClient
    for Client<'_, W, RES_CAPACITY, URC_CAPACITY>
where
    W: Write,
{
    fn send<A: AtatCmd<LEN>, const LEN: usize>(&mut self, cmd: &A) -> Result<A::Response, Error> {
        self.wait_cooldown_timer();

        let cmd_bytes = cmd.as_bytes();
        let cmd_slice = cmd.get_slice(&cmd_bytes);
        if cmd_slice.len() < 50 {
            debug!("Sending command: {:?}", LossyStr(cmd_slice));
        } else {
            debug!(
                "Sending command with long payload ({} bytes)",
                cmd_slice.len(),
            );
        }

        self.writer
            .write_all(&cmd_slice)
            .map_err(|_e| Error::Write)?;
        self.writer.flush().map_err(|_e| Error::Write)?;

        if !A::EXPECTS_RESPONSE_CODE {
            debug!("Command does not expect a response");
            self.start_cooldown_timer();
            return cmd.parse(Ok(&[]));
        }

        let response = Timer::with_timeout(Duration::from_millis(A::MAX_TIMEOUT_MS.into()), || {
            self.res_reader.read().map(|mut grant| {
                grant.auto_release(true);

                let frame = Frame::decode(grant.as_ref());
                let resp = match Response::from(frame) {
                    Response::Result(r) => r,
                    Response::Prompt(_) => Ok(&[] as &[u8]),
                };

                cmd.parse(resp)
            })
        });

        self.start_cooldown_timer();
        response
    }

    fn try_read_urc_with<Urc: AtatUrc, F: for<'b> FnOnce(Urc::Response, &'b [u8]) -> bool>(
        &mut self,
        handle: F,
    ) -> bool {
        if let Some(urc_grant) = self.urc_reader.read() {
            self.start_cooldown_timer();
            if let Some(urc) = Urc::parse(&urc_grant) {
                if handle(urc, &urc_grant) {
                    urc_grant.release();
                    return true;
                }
            } else {
                error!("Parsing URC FAILED: {:?}", LossyStr(&urc_grant));
                urc_grant.release();
            }
        }

        false
    }

    fn max_urc_len() -> usize {
        // bbqueue can only guarantee grant sizes of half its capacity if the queue is empty.
        // A _frame_ grant returned by bbqueue has a header. Assume that it is 2 bytes.
        (URC_CAPACITY / 2) - 2
    }
}

#[cfg(test)]
mod test {
    use crate::frame::FrameProducerExt;

    use super::*;
    use crate::atat_derive::{AtatCmd, AtatEnum, AtatResp, AtatUrc};
    use crate::{self as atat, InternalError};
    use bbqueue::BBBuffer;
    use heapless::String;
    use serde_at::HexStr;

    const TEST_RX_BUF_LEN: usize = 256;
    const TEST_RES_CAPACITY: usize = 3 * TEST_RX_BUF_LEN;
    const TEST_URC_CAPACITY: usize = 3 * TEST_RX_BUF_LEN;

    #[derive(Debug)]
    pub struct IoError;

    impl embedded_io::Error for IoError {
        fn kind(&self) -> embedded_io::ErrorKind {
            embedded_io::ErrorKind::Other
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

    impl embedded_io::Io for TxMock {
        type Error = IoError;
    }

    impl embedded_io::blocking::Write for TxMock {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            for c in buf {
                self.s.push(*c as char).map_err(|_| IoError)?;
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
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
        ($config:expr) => {{
            static mut RES_Q: BBBuffer<TEST_RES_CAPACITY> = BBBuffer::new();
            let (res_p, res_c) = unsafe { RES_Q.try_split_framed().unwrap() };

            static mut URC_Q: BBBuffer<TEST_URC_CAPACITY> = BBBuffer::new();
            let (urc_p, urc_c) = unsafe { URC_Q.try_split_framed().unwrap() };

            let tx_mock = TxMock::new(String::new());
            let client: Client<TxMock, TEST_RES_CAPACITY, TEST_URC_CAPACITY> =
                Client::new(tx_mock, res_c, urc_c, $config);
            (client, res_p, urc_p)
        }};
    }

    #[test]
    fn error_response() {
        let (mut client, mut p, _) = setup!(Config::new());

        let cmd = ErrorTester { x: 7 };

        p.try_enqueue(Err(InternalError::Error).into()).unwrap();

        assert_eq!(client.send(&cmd), Err(Error::Error));
    }

    #[test]
    fn generic_error_response() {
        let (mut client, mut p, _) = setup!(Config::new());

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        p.try_enqueue(Err(InternalError::Error).into()).unwrap();

        assert_eq!(client.send(&cmd), Err(Error::Error));
    }

    #[test]
    fn string_sent() {
        let (mut client, mut p, _) = setup!(Config::new());

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        p.try_enqueue(Frame::Response(&[])).unwrap();

        assert_eq!(client.send(&cmd), Ok(NoResponse));

        assert_eq!(
            client.writer.s,
            String::<32>::from("AT+CFUN=4,0\r\n"),
            "Wrong encoding of string"
        );

        p.try_enqueue(Frame::Response(&[])).unwrap();

        let cmd = Test2Cmd {
            fun: Functionality::DM,
            rst: Some(ResetMode::Reset),
        };
        assert_eq!(client.send(&cmd), Ok(NoResponse));

        assert_eq!(
            client.writer.s,
            String::<32>::from("AT+CFUN=4,0\r\nAT+FUN=1,6\r\n"),
            "Reverse order string did not match"
        );
    }

    #[test]
    fn blocking() {
        let (mut client, mut p, _) = setup!(Config::new());

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        p.try_enqueue(Frame::Response(&[])).unwrap();

        assert_eq!(client.send(&cmd), Ok(NoResponse));
        assert_eq!(client.writer.s, String::<32>::from("AT+CFUN=4,0\r\n"));
    }

    // Test response containing string
    #[test]
    fn response_string() {
        let (mut client, mut p, _) = setup!(Config::new());

        // String last
        let cmd = TestRespStringCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let response = b"+CUN: 22,16,\"0123456789012345\"";
        p.try_enqueue(Frame::Response(response)).unwrap();

        assert_eq!(
            client.send(&cmd),
            Ok(TestResponseString {
                socket: 22,
                length: 16,
                data: String::<64>::from("0123456789012345")
            })
        );

        // Mixed order for string
        let cmd = TestRespStringMixCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let response = b"+CUN: \"0123456789012345\",22,16";
        p.try_enqueue(Frame::Response(response)).unwrap();

        assert_eq!(
            client.send(&cmd),
            Ok(TestResponseStringMixed {
                socket: 22,
                length: 16,
                data: String::<64>::from("0123456789012345")
            })
        );
    }

    #[test]
    fn urc() {
        let (mut client, _, mut urc_p) = setup!(Config::new());

        let response = b"+UMWI: 0, 1";

        let mut grant = urc_p.grant(response.len()).unwrap();
        grant.copy_from_slice(response.as_ref());
        grant.commit(response.len());

        assert!(client.try_read_urc::<Urc>().is_some());
    }

    #[test]
    fn urc_keyword() {
        let (mut client, _, mut urc_p) = setup!(Config::new());

        let response = b"CONNECT OK";

        let mut grant = urc_p.grant(response.len()).unwrap();
        grant.copy_from_slice(response.as_ref());
        grant.commit(response.len());

        assert_eq!(Urc::ConnectOk, client.try_read_urc::<Urc>().unwrap());
    }

    #[test]
    fn invalid_response() {
        let (mut client, mut p, _) = setup!(Config::new());

        // String last
        let cmd = TestRespStringCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let response = b"+CUN: 22,16,22";
        p.try_enqueue(Frame::Response(response)).unwrap();

        assert_eq!(client.send(&cmd), Err(Error::Parse));
    }

    #[test]
    fn quote_and_no_quote_strings() {
        #[derive(Clone, PartialEq, AtatCmd)]
        #[at_cmd("+DEVEUI", NoResponse)]
        pub struct WithQuoteNoValHexStr {
            pub val: HexStr<u16>,
        }

        let val = HexStr {
            val: 0xA0F5,
            ..Default::default()
        };
        let val = WithQuoteNoValHexStr { val };
        let b = val.as_bytes();
        let s = core::str::from_utf8(&b).unwrap();
        assert_eq!(s, "AT+DEVEUI=\"A0F5\"\r\n");

        #[derive(Clone, PartialEq, AtatCmd)]
        #[at_cmd("+DEVEUI", NoResponse, quote_escape_strings = true)]
        pub struct WithQuoteHexStr {
            pub val: HexStr<u16>,
        }

        let val = HexStr {
            val: 0xA0F5,
            ..Default::default()
        };
        let val = WithQuoteHexStr { val };
        let b = val.as_bytes();
        let s = core::str::from_utf8(&b).unwrap();
        assert_eq!(s, "AT+DEVEUI=\"A0F5\"\r\n");

        #[derive(Clone, PartialEq, AtatCmd)]
        #[at_cmd("+DEVEUI", NoResponse, quote_escape_strings = false)]
        pub struct WithoutQuoteHexStr {
            pub val: HexStr<u128>,
        }

        let val = HexStr {
            val: 0xA0F5_A0F5_A0F5_A0F5_A0F5_A0F5_A0F5_A0F5,
            ..Default::default()
        };
        let val = WithoutQuoteHexStr { val };
        let b = val.as_bytes();
        let s = core::str::from_utf8(&b).unwrap();
        assert_eq!(s, "AT+DEVEUI=A0F5A0F5A0F5A0F5A0F5A0F5A0F5A0F5\r\n");
    }

    // #[test]
    // fn tx_timeout() {
    //     let timeout = Duration::from_millis(20);
    //     let (mut client, mut p, _) = setup!(Config::new().tx_timeout(1));

    //     let cmd = SetModuleFunctionality {
    //         fun: Functionality::APM,
    //         rst: Some(ResetMode::DontReset),
    //     };

    //     p.try_enqueue(Frame::Response(&[])).unwrap();

    //     assert_eq!(client.send(&cmd), Err(Error::Timeout));
    // }

    // #[test]
    // fn flush_timeout() {
    //     let timeout = Duration::from_millis(20);
    //     let (mut client, mut p, _) = setup!(Config::new().flush_timeout(1));

    //     let cmd = SetModuleFunctionality {
    //         fun: Functionality::APM,
    //         rst: Some(ResetMode::DontReset),
    //     };

    //     p.try_enqueue(Frame::Response(&[])).unwrap();

    //     assert_eq!(client.send(&cmd), Err(Error::Timeout));
    // }
}
