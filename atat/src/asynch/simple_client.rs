use super::AtatClient;
use crate::{helpers::LossyStr, AtatCmd, Config, DigestResult, Digester, Error, Response};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_io_async::{Read, Write};

pub struct SimpleClient<'a, RW: Read + Write, D: Digester> {
    rw: RW,
    digester: D,
    buf: &'a mut [u8],
    pos: usize,
    config: Config,
    cooldown_timer: Option<Timer>,
}

impl<'a, RW: Read + Write, D: Digester> SimpleClient<'a, RW, D> {
    pub fn new(rw: RW, digester: D, buf: &'a mut [u8], config: Config) -> Self {
        Self {
            rw,
            digester,
            buf,
            config,
            pos: 0,
            cooldown_timer: None,
        }
    }

    async fn send_request(&mut self, len: usize) -> Result<(), Error> {
        if len < 50 {
            debug!("Sending command: {:?}", LossyStr(&self.buf[..len]));
        } else {
            debug!("Sending command with long payload ({} bytes)", len);
        }

        self.wait_cooldown_timer().await;

        // Write request
        with_timeout(self.config.tx_timeout, self.rw.write_all(&self.buf[..len]))
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(|_| Error::Write)?;

        with_timeout(self.config.flush_timeout, self.rw.flush())
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(|_| Error::Write)?;

        self.start_cooldown_timer();
        Ok(())
    }

    async fn wait_response<'guard>(&'guard mut self) -> Result<Response<256>, Error> {
        loop {
            match self.rw.read(&mut self.buf[self.pos..]).await {
                Ok(n) => {
                    self.pos += n;
                }
                _ => return Err(Error::Read),
            };

            trace!("Buffer contents: '{:?}'", LossyStr(&self.buf[..self.pos]));

            while self.pos > 0 {
                let (res, swallowed) = match self.digester.digest(&self.buf[..self.pos]) {
                    (DigestResult::None, swallowed) => {
                        if swallowed > 0 {
                            debug!(
                                "Received echo or whitespace ({}/{}): {:?}",
                                swallowed,
                                self.pos,
                                LossyStr(&self.buf[..swallowed])
                            );
                        }
                        (None, swallowed)
                    }
                    (DigestResult::Urc(urc_line), swallowed) => {
                        warn!("Unable to handle URC! Ignoring: {:?}", LossyStr(urc_line));
                        (None, swallowed)
                    }
                    (DigestResult::Prompt(prompt), swallowed) => {
                        debug!("Received prompt ({}/{})", swallowed, self.pos);

                        (Some(Response::Prompt(prompt)), swallowed)
                    }
                    (DigestResult::Response(resp), swallowed) => {
                        match &resp {
                            Ok(r) => {
                                if r.is_empty() {
                                    debug!("Received OK ({}/{})", swallowed, self.pos)
                                } else {
                                    debug!(
                                        "Received response ({}/{}): {:?}",
                                        swallowed,
                                        self.pos,
                                        LossyStr(r)
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Received error response ({}/{}): {:?}",
                                    swallowed, self.pos, e
                                );
                            }
                        }

                        (Some(resp.into()), swallowed)
                    }
                };

                if swallowed == 0 {
                    break;
                }

                self.consume(swallowed);

                if let Some(resp) = res {
                    return Ok(resp);
                }
            }
        }
    }

    fn consume(&mut self, amt: usize) {
        self.buf.copy_within(amt..self.pos, 0);
        self.pos -= amt;
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

impl<RW: Read + Write, D: Digester> AtatClient for SimpleClient<'_, RW, D> {
    async fn send<Cmd: AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, Error> {
        let len = cmd.write(&mut self.buf);

        self.send_request(len).await?;
        if !Cmd::EXPECTS_RESPONSE_CODE {
            cmd.parse(Ok(&[]))
        } else {
            let response = embassy_time::with_timeout(
                Duration::from_millis(Cmd::MAX_TIMEOUT_MS.into()),
                self.wait_response(),
            )
            .await
            .map_err(|_| Error::Timeout)??;

            cmd.parse((&response).into())
        }
    }
}
