use embassy_time::Duration;
use embedded_io::blocking::Write;

use super::{timer::Timer, AtatClient};
use crate::{helpers::LossyStr, reschannel::ResChannel, AtatCmd, Config, Error, Response};

/// Client responsible for handling send, receive and timeout from the
/// userfacing side. The client is decoupled from the ingress-manager through
/// some spsc queue consumers, where any received responses can be dequeued. The
/// Client also has an spsc producer, to allow signaling commands like
/// `reset` to the ingress-manager.
pub struct Client<'a, W, const INGRESS_BUF_SIZE: usize>
where
    W: Write,
{
    writer: W,
    res_channel: &'a ResChannel<INGRESS_BUF_SIZE>,
    cooldown_timer: Option<Timer>,
    config: Config,
}

impl<'a, W, const INGRESS_BUF_SIZE: usize> Client<'a, W, INGRESS_BUF_SIZE>
where
    W: Write,
{
    pub(crate) fn new(
        writer: W,
        res_channel: &'a ResChannel<INGRESS_BUF_SIZE>,
        config: Config,
    ) -> Self {
        Self {
            writer,
            res_channel,
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

impl<W, const INGRESS_BUF_SIZE: usize> AtatClient for Client<'_, W, INGRESS_BUF_SIZE>
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

        let mut response_subscription = self.res_channel.subscriber().unwrap();

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
            response_subscription.try_next_message_pure().map(|frame| {
                let resp = match Response::from(&frame) {
                    Response::Result(r) => r,
                    Response::Prompt(_) => Ok(&[] as &[u8]),
                };

                cmd.parse(resp)
            })
        });

        self.start_cooldown_timer();
        response
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::atat_derive::{AtatCmd, AtatEnum, AtatResp, AtatUrc};
    use crate::reschannel::ResMessage;
    use crate::{self as atat, InternalError};
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use embassy_sync::pubsub::{PubSubChannel, Publisher};
    use heapless::String;

    const TEST_RX_BUF_LEN: usize = 256;

    #[derive(Debug)]
    pub struct IoError;

    impl embedded_io::Error for IoError {
        fn kind(&self) -> embedded_io::ErrorKind {
            embedded_io::ErrorKind::Other
        }
    }

    struct TxMock<'a> {
        buf: String<64>,
        publisher: Publisher<'a, CriticalSectionRawMutex, String<64>, 1, 1, 1>,
    }

    impl<'a> TxMock<'a> {
        fn new(publisher: Publisher<'a, CriticalSectionRawMutex, String<64>, 1, 1, 1>) -> Self {
            TxMock {
                buf: String::new(),
                publisher,
            }
        }
    }

    impl embedded_io::Io for TxMock<'_> {
        type Error = IoError;
    }

    impl embedded_io::blocking::Write for TxMock<'_> {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            for c in buf {
                self.buf.push(*c as char).map_err(|_| IoError)?;
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            self.publisher.try_publish(self.buf.clone()).unwrap();
            self.buf.clear();
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
            static TX_CHANNEL: PubSubChannel<CriticalSectionRawMutex, String<64>, 1, 1, 1> =
                PubSubChannel::new();
            static RES_CHANNEL: ResChannel<TEST_RX_BUF_LEN> = ResChannel::new();

            let tx_mock = TxMock::new(TX_CHANNEL.publisher().unwrap());
            let client: Client<TxMock, TEST_RX_BUF_LEN> =
                Client::new(tx_mock, &RES_CHANNEL, $config);
            (
                client,
                TX_CHANNEL.subscriber().unwrap(),
                RES_CHANNEL.publisher().unwrap(),
            )
        }};
    }

    #[tokio::test]
    async fn error_response() {
        let (mut client, mut tx, rx) = setup!(Config::new());

        let cmd = ErrorTester { x: 7 };

        let sent = tokio::spawn(async move {
            tx.next_message_pure().await;
            rx.try_publish(Err(InternalError::Error).into()).unwrap();
        });

        tokio::task::spawn_blocking(move || {
            assert_eq!(Err(Error::Error), client.send(&cmd));
        })
        .await
        .unwrap();

        sent.await.unwrap();
    }

    #[tokio::test]
    async fn generic_error_response() {
        let (mut client, mut tx, rx) = setup!(Config::new());

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let sent = tokio::spawn(async move {
            tx.next_message_pure().await;
            rx.try_publish(Err(InternalError::Error).into()).unwrap();
        });

        tokio::task::spawn_blocking(move || {
            assert_eq!(Err(Error::Error), client.send(&cmd));
        })
        .await
        .unwrap();

        sent.await.unwrap();
    }

    #[tokio::test]
    async fn string_sent() {
        let (mut client, mut tx, rx) = setup!(Config::new());

        let cmd0 = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let cmd1 = Test2Cmd {
            fun: Functionality::DM,
            rst: Some(ResetMode::Reset),
        };

        let sent = tokio::spawn(async move {
            let sent0 = tx.next_message_pure().await;
            rx.try_publish(ResMessage::empty_response()).unwrap();

            let sent1 = tx.next_message_pure().await;
            rx.try_publish(ResMessage::empty_response()).unwrap();

            (sent0, sent1)
        });

        tokio::task::spawn_blocking(move || {
            assert_eq!(client.send(&cmd0), Ok(NoResponse));
            assert_eq!(client.send(&cmd1), Ok(NoResponse));
        })
        .await
        .unwrap();

        let (sent0, sent1) = sent.await.unwrap();
        assert_eq!("AT+CFUN=4,0\r\n", &sent0);
        assert_eq!("AT+FUN=1,6\r\n", &sent1);
    }

    #[tokio::test]
    async fn blocking() {
        let (mut client, mut tx, rx) = setup!(Config::new());

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let sent = tokio::spawn(async move {
            let sent = tx.next_message_pure().await;
            rx.try_publish(ResMessage::empty_response()).unwrap();
            sent
        });

        tokio::task::spawn_blocking(move || {
            assert_eq!(client.send(&cmd), Ok(NoResponse));
        })
        .await
        .unwrap();

        let sent = sent.await.unwrap();
        assert_eq!("AT+CFUN=4,0\r\n", &sent);
    }

    // Test response containing string
    #[tokio::test]
    async fn response_string() {
        let (mut client, mut tx, rx) = setup!(Config::new());

        // String last
        let cmd0 = TestRespStringCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };
        let response0 = b"+CUN: 22,16,\"0123456789012345\"";

        // Mixed order for string
        let cmd1 = TestRespStringMixCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };
        let response1 = b"+CUN: \"0123456789012345\",22,16";

        let sent = tokio::spawn(async move {
            let sent0 = tx.next_message_pure().await;
            rx.try_publish(ResMessage::response(response0)).unwrap();

            let sent1 = tx.next_message_pure().await;
            rx.try_publish(ResMessage::response(response1)).unwrap();

            (sent0, sent1)
        });

        tokio::task::spawn_blocking(move || {
            assert_eq!(
                Ok(TestResponseString {
                    socket: 22,
                    length: 16,
                    data: String::<64>::from("0123456789012345")
                }),
                client.send(&cmd0),
            );
            assert_eq!(
                Ok(TestResponseStringMixed {
                    socket: 22,
                    length: 16,
                    data: String::<64>::from("0123456789012345")
                }),
                client.send(&cmd1),
            );
        })
        .await
        .unwrap();

        sent.await.unwrap();
    }

    #[tokio::test]
    async fn invalid_response() {
        let (mut client, mut tx, rx) = setup!(Config::new());

        // String last
        let cmd = TestRespStringCmd {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let sent = tokio::spawn(async move {
            tx.next_message_pure().await;
            rx.try_publish(ResMessage::response(b"+CUN: 22,16,22"))
                .unwrap();
        });

        tokio::task::spawn_blocking(move || {
            assert_eq!(Err(Error::Parse), client.send(&cmd));
        })
        .await
        .unwrap();

        sent.await.unwrap();
    }

    // #[test]
    // fn tx_timeout() {
    //     let timeout = Duration::from_millis(20);
    //     let (mut client, mut p) = setup!(Config::new().tx_timeout(1));

    //     let cmd = SetModuleFunctionality {
    //         fun: Functionality::APM,
    //         rst: Some(ResetMode::DontReset),
    //     };

    //     p.try_enqueue(Frame::empty_response()).unwrap();

    //     assert_eq!(client.send(&cmd), Err(Error::Timeout));
    // }

    // #[test]
    // fn flush_timeout() {
    //     let timeout = Duration::from_millis(20);
    //     let (mut client, mut p) = setup!(Config::new().flush_timeout(1));

    //     let cmd = SetModuleFunctionality {
    //         fun: Functionality::APM,
    //         rst: Some(ResetMode::DontReset),
    //     };

    //     p.try_enqueue(Frame::empty_response()).unwrap();

    //     assert_eq!(client.send(&cmd), Err(Error::Timeout));
    // }
}
