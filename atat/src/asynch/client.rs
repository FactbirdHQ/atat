use super::AtatClient;
use crate::{
    helpers::LossyStr, response_channel::ResponseChannel, AtatCmd, Config, Error, Response,
};
use embassy_time::{Duration, Instant, TimeoutError, Timer};
use embedded_io::asynch::Write;
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
    pub(crate) fn new(
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
                    if new_expires == expires {
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
