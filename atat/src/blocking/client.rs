use embassy_time::{Duration, Instant, TimeoutError};
use embedded_io::Write;

use super::{blocking_timer::BlockingTimer, AtatClient};
use crate::{
    helpers::LossyStr, response_channel::ResponseChannel, AtatCmd, Config, Error, Response,
};

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
    res_channel: &'a ResponseChannel<INGRESS_BUF_SIZE>,
    cooldown_timer: Option<BlockingTimer>,
    config: Config,
}

impl<'a, W, const INGRESS_BUF_SIZE: usize> Client<'a, W, INGRESS_BUF_SIZE>
where
    W: Write,
{
    pub fn new(
        writer: W,
        res_channel: &'a ResponseChannel<INGRESS_BUF_SIZE>,
        config: Config,
    ) -> Self {
        Self {
            writer,
            res_channel,
            cooldown_timer: None,
            config,
        }
    }

    fn send_command(&mut self, cmd: &[u8]) -> Result<(), Error> {
        self.wait_cooldown_timer();

        self.send_inner(cmd)?;

        self.start_cooldown_timer();
        Ok(())
    }

    fn send_request(
        &mut self,
        cmd: &[u8],
        timeout: Duration,
    ) -> Result<Response<INGRESS_BUF_SIZE>, Error> {
        self.wait_cooldown_timer();

        let mut response_subscription = self.res_channel.subscriber().unwrap();
        self.send_inner(cmd)?;

        let response = self
            .with_timeout(timeout, || response_subscription.try_next_message_pure())
            .map_err(|_| Error::Timeout);

        self.start_cooldown_timer();
        response
    }

    fn send_inner(&mut self, cmd: &[u8]) -> Result<(), Error> {
        if cmd.len() < 50 {
            debug!("Sending command: {:?}", LossyStr(cmd));
        } else {
            debug!("Sending command with long payload ({} bytes)", cmd.len(),);
        }

        self.writer.write_all(cmd).map_err(|_| Error::Write)?;
        self.writer.flush().map_err(|_| Error::Write)?;
        Ok(())
    }

    fn with_timeout<R>(
        &self,
        timeout: Duration,
        mut poll: impl FnMut() -> Option<R>,
    ) -> Result<R, TimeoutError> {
        let start = Instant::now();

        loop {
            if let Some(res) = poll() {
                return Ok(res);
            }
            if (self.config.get_response_timeout)(start, timeout) <= Instant::now() {
                return Err(TimeoutError);
            }
        }
    }

    fn start_cooldown_timer(&mut self) {
        self.cooldown_timer = Some(BlockingTimer::after(self.config.cmd_cooldown));
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
    fn send<Cmd: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &Cmd,
    ) -> Result<Cmd::Response, Error> {
        let cmd_vec = cmd.as_bytes();
        let cmd_slice = cmd.get_slice(&cmd_vec);
        if !Cmd::EXPECTS_RESPONSE_CODE {
            self.send_command(cmd_slice)?;
            cmd.parse(Ok(&[]))
        } else {
            let response =
                self.send_request(cmd_slice, Duration::from_millis(Cmd::MAX_TIMEOUT_MS.into()))?;
            cmd.parse((&response).into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::atat_derive::{AtatCmd, AtatEnum, AtatResp, AtatUrc};
    use crate::{self as atat, InternalError, Response};
    use core::sync::atomic::{AtomicU64, Ordering};
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use embassy_sync::pubsub::PubSubChannel;
    use embassy_time::Timer;
    use heapless::String;

    const TEST_RX_BUF_LEN: usize = 256;

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
            static RES_CHANNEL: ResponseChannel<TEST_RX_BUF_LEN> = ResponseChannel::new();

            let tx_mock = crate::tx_mock::TxMock::new(TX_CHANNEL.publisher().unwrap());
            let client: Client<crate::tx_mock::TxMock, TEST_RX_BUF_LEN> =
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
            rx.try_publish(Response::default()).unwrap();

            let sent1 = tx.next_message_pure().await;
            rx.try_publish(Response::default()).unwrap();

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
            rx.try_publish(Response::default()).unwrap();
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
            rx.try_publish(Response::ok(response0)).unwrap();

            let sent1 = tx.next_message_pure().await;
            rx.try_publish(Response::ok(response1)).unwrap();

            (sent0, sent1)
        });

        tokio::task::spawn_blocking(move || {
            assert_eq!(
                Ok(TestResponseString {
                    socket: 22,
                    length: 16,
                    data: String::<64>::try_from("0123456789012345").unwrap()
                }),
                client.send(&cmd0),
            );
            assert_eq!(
                Ok(TestResponseStringMixed {
                    socket: 22,
                    length: 16,
                    data: String::<64>::try_from("0123456789012345").unwrap()
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
            rx.try_publish(Response::ok(b"+CUN: 22,16,22")).unwrap();
        });

        tokio::task::spawn_blocking(move || {
            assert_eq!(Err(Error::Parse), client.send(&cmd));
        })
        .await
        .unwrap();

        sent.await.unwrap();
    }

    #[tokio::test]
    async fn custom_timeout() {
        static CALL_COUNT: AtomicU64 = AtomicU64::new(0);

        fn custom_response_timeout(sent: Instant, timeout: Duration) -> Instant {
            CALL_COUNT.fetch_add(1, Ordering::Relaxed);
            assert_eq!(
                Duration::from_millis(SetModuleFunctionality::MAX_TIMEOUT_MS.into()),
                timeout
            );
            // Effectively ignoring the timeout configured for the command
            // The default response timeout is "sent + timeout"
            sent + Duration::from_millis(100)
        }

        let (mut client, mut tx, _rx) =
            setup!(Config::new().get_response_timeout(custom_response_timeout));

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let sent = tokio::spawn(async move {
            tx.next_message_pure().await;
            // Do not emit a response effectively causing a timeout
        });

        tokio::task::spawn_blocking(move || {
            assert_eq!(Err(Error::Timeout), client.send(&cmd));
        })
        .await
        .unwrap();

        sent.await.unwrap();

        assert_ne!(0, CALL_COUNT.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn custom_timeout_modified_during_request() {
        static CALL_COUNT: AtomicU64 = AtomicU64::new(0);

        fn custom_response_timeout(sent: Instant, timeout: Duration) -> Instant {
            CALL_COUNT.fetch_add(1, Ordering::Relaxed);
            assert_eq!(
                Duration::from_millis(SetModuleFunctionality::MAX_TIMEOUT_MS.into()),
                timeout
            );
            // Effectively ignoring the timeout configured for the command
            // The default response timeout is "sent + timeout"
            // Let the timeout instant be extended depending on the current time
            if Instant::now() < sent + Duration::from_millis(100) {
                // Initial timeout
                sent + Duration::from_millis(200)
            } else {
                // Extended timeout
                sent + Duration::from_millis(500)
            }
        }

        let (mut client, mut tx, rx) =
            setup!(Config::new().get_response_timeout(custom_response_timeout));

        let cmd = SetModuleFunctionality {
            fun: Functionality::APM,
            rst: Some(ResetMode::DontReset),
        };

        let sent = tokio::spawn(async move {
            tx.next_message_pure().await;
            // Emit response in the extended timeout timeframe
            Timer::after(Duration::from_millis(300)).await;
            rx.try_publish(Response::default()).unwrap();
        });

        tokio::task::spawn_blocking(move || {
            assert_eq!(Ok(NoResponse), client.send(&cmd));
        })
        .await
        .unwrap();

        sent.await.unwrap();

        assert_ne!(0, CALL_COUNT.load(Ordering::Relaxed));
    }

    // #[test]
    // fn tx_timeout() {
    //     let timeout = Duration::from_millis(20);
    //     let (mut client, mut p) = setup!(Config::new().tx_timeout(1));

    //     let cmd = SetModuleFunctionality {
    //         fun: Functionality::APM,
    //         rst: Some(ResetMode::DontReset),
    //     };

    //     p.try_enqueue(Frame::default()).unwrap();

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

    //     p.try_enqueue(Frame::default()).unwrap();

    //     assert_eq!(client.send(&cmd), Err(Error::Timeout));
    // }
}
