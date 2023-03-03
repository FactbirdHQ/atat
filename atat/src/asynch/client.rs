use super::AtatClient;
use crate::{frame::Frame, helpers::LossyStr, AtatCmd, Config, Error, Response};
use bbqueue::framed::FrameConsumer;
use embassy_time::{with_timeout, Duration, Timer};
use embedded_io::asynch::Write;

pub struct Client<'a, W: Write, const INGRESS_BUF_SIZE: usize, const RES_CAPACITY: usize> {
    writer: W,
    res_reader: FrameConsumer<'a, RES_CAPACITY>,
    config: Config,
    cooldown_timer: Option<Timer>,
}

impl<'a, W: Write, const INGRESS_BUF_SIZE: usize, const RES_CAPACITY: usize>
    Client<'a, W, INGRESS_BUF_SIZE, RES_CAPACITY>
{
    pub(crate) fn new(
        writer: W,
        res_reader: FrameConsumer<'a, RES_CAPACITY>,
        config: Config,
    ) -> Self {
        Self {
            writer,
            res_reader,
            config,
            cooldown_timer: None,
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

impl<W: Write, const INGRESS_BUF_SIZE: usize, const RES_CAPACITY: usize> AtatClient
    for Client<'_, W, INGRESS_BUF_SIZE, RES_CAPACITY>
{
    async fn send<Cmd: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &Cmd,
    ) -> Result<Cmd::Response, Error> {
        self.wait_cooldown_timer().await;

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
            .write_all(cmd_slice)
            .await
            .map_err(|_| Error::Write)?;

        self.writer.flush().await.map_err(|_| Error::Write)?;

        if !Cmd::EXPECTS_RESPONSE_CODE {
            debug!("Command does not expect a response");
            self.start_cooldown_timer();
            return cmd.parse(Ok(&[]));
        }

        let response = match with_timeout(
            Duration::from_millis(Cmd::MAX_TIMEOUT_MS.into()),
            self.res_reader.read_async(),
        )
        .await
        {
            Ok(res) => {
                let mut grant = res.unwrap();
                grant.auto_release(true);

                let frame = Frame::decode(grant.as_ref());
                let resp = match Response::from(frame) {
                    Response::Result(r) => r,
                    Response::Prompt(_) => Ok(&[][..]),
                };

                cmd.parse(resp)
            }
            Err(_) => {
                warn!("Received timeout after {}ms", Cmd::MAX_TIMEOUT_MS);
                Err(Error::Timeout)
            }
        };

        self.start_cooldown_timer();
        response
    }

    fn max_response_len() -> usize {
        INGRESS_BUF_SIZE
    }
}
