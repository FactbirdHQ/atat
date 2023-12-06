use super::AtatClient;
use crate::{
    helpers::LossyStr, response_channel::ResponseChannel, AtatCmd, Config, Error, Response,
};
use embassy_time::{Duration, Instant, TimeoutError, Timer};
use embedded_io_async::Write;
use futures::{
    future::{select, Either},
    pin_mut, Future,
};

pub struct Client<'a, W: Write, const INGRESS_BUF_SIZE: usize> {
    writer: W,
    res_channel: &'a ResponseChannel<INGRESS_BUF_SIZE>,
    config: Config,
    cooldown_timer: Option<Timer>,
}

impl<'a, W: Write, const INGRESS_BUF_SIZE: usize> Client<'a, W, INGRESS_BUF_SIZE> {
    pub fn new(
        writer: W,
        res_channel: &'a ResponseChannel<INGRESS_BUF_SIZE>,
        config: Config,
    ) -> Self {
        Self {
            writer,
            res_channel,
            config,
            cooldown_timer: None,
        }
    }

    async fn send_command(&mut self, cmd: &[u8]) -> Result<(), Error> {
        self.wait_cooldown_timer().await;

        self.send_inner(cmd).await?;

        self.start_cooldown_timer();
        Ok(())
    }

    async fn send_request(
        &mut self,
        cmd: &[u8],
        timeout: Duration,
    ) -> Result<Response<INGRESS_BUF_SIZE>, Error> {
        self.wait_cooldown_timer().await;

        let mut response_subscription = self.res_channel.subscriber().unwrap();
        self.send_inner(cmd).await?;

        let response = self
            .with_timeout(timeout, response_subscription.next_message_pure())
            .await
            .map_err(|_| Error::Timeout);

        self.start_cooldown_timer();
        response
    }

    async fn send_inner(&mut self, cmd: &[u8]) -> Result<(), Error> {
        if cmd.len() < 50 {
            debug!("Sending command: {:?}", LossyStr(cmd));
        } else {
            debug!("Sending command with long payload ({} bytes)", cmd.len(),);
        }

        self.writer.write_all(cmd).await.map_err(|_| Error::Write)?;
        self.writer.flush().await.map_err(|_| Error::Write)?;
        Ok(())
    }

    async fn with_timeout<F: Future>(
        &self,
        timeout: Duration,
        fut: F,
    ) -> Result<F::Output, TimeoutError> {
        let start = Instant::now();
        let mut expires = (self.config.get_response_timeout)(start, timeout);

        pin_mut!(fut);

        loop {
            fut = match select(fut, Timer::at(expires)).await {
                Either::Left((r, _)) => return Ok(r),
                Either::Right((_, fut)) => {
                    let new_expires = (self.config.get_response_timeout)(start, timeout);
                    if new_expires <= expires {
                        return Err(TimeoutError);
                    }
                    expires = new_expires;
                    fut
                }
            };
        }
    }

    fn start_cooldown_timer(&mut self) {
        self.cooldown_timer = Some(Timer::after(self.config.cmd_cooldown));
    }

    async fn wait_cooldown_timer(&mut self) {
        if let Some(cooldown) = self.cooldown_timer.take() {
            cooldown.await
        }
    }
}

impl<W: Write, const INGRESS_BUF_SIZE: usize> AtatClient for Client<'_, W, INGRESS_BUF_SIZE> {
    async fn send<Cmd: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &Cmd,
    ) -> Result<Cmd::Response, Error> {
        let cmd_vec = cmd.as_bytes();
        let cmd_slice = cmd.get_slice(&cmd_vec);
        if !Cmd::EXPECTS_RESPONSE_CODE {
            self.send_command(cmd_slice).await?;
            cmd.parse(Ok(&[]))
        } else {
            let response = self
                .send_request(cmd_slice, Duration::from_millis(Cmd::MAX_TIMEOUT_MS.into()))
                .await?;
            cmd.parse((&response).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as atat;
    use crate::atat_derive::{AtatCmd, AtatEnum, AtatResp};
    use crate::{Error, Response};
    use core::sync::atomic::{AtomicU64, Ordering};
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use embassy_sync::pubsub::PubSubChannel;
    use embassy_time::{Duration, Instant, Timer};
    use heapless::String;
    use tokio::join;

    const TEST_RX_BUF_LEN: usize = 256;

    #[derive(Clone, AtatCmd)]
    #[at_cmd("+CFUN", NoResponse, timeout_ms = 180000)]
    pub struct SetModuleFunctionality {
        #[at_arg(position = 0)]
        pub fun: Functionality,
        #[at_arg(position = 1)]
        pub rst: Option<ResetMode>,
    }

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

        let send = tokio::task::spawn(async move {
            assert_eq!(Err(Error::Timeout), client.send(&cmd).await);
        });

        let (sent, send) = join!(sent, send);
        sent.unwrap();
        send.unwrap();

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

        let send = tokio::task::spawn(async move {
            assert_eq!(Ok(NoResponse), client.send(&cmd).await);
        });

        let (sent, send) = join!(sent, send);
        sent.unwrap();
        send.unwrap();

        assert_ne!(0, CALL_COUNT.load(Ordering::Relaxed));
    }
}
