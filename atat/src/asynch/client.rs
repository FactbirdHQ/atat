use crate::{Config, helpers::LossyStr, AtatCmd, AtatUrc, Error, Response, frame::Frame};
use bbqueue::framed::FrameConsumer;
use embedded_hal_async::delay::DelayUs;
use embedded_io::asynch::Write;
use embedded_time::{
    duration::Milliseconds,
    timer::param::{OneShot, Running},
    Clock, Timer,
};
use futures::{
    future::{select, Either},
    pin_mut,
};

pub trait AtatClient {
    async fn send<Cmd: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &Cmd,
    ) -> Result<Cmd::Response, Error>;

    async fn send_retry<Cmd: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &Cmd,
    ) -> Result<Cmd::Response, Error> {
        for attempt in 1..=Cmd::ATTEMPTS {
            if attempt > 1 {
                debug!("Attempt {}:", attempt);
            }

            match self.send(cmd).await {
                Err(Error::Timeout) => {}
                r => return r,
            }
        }
        Err(Error::Timeout)
    }

    fn try_read_urc<Urc: AtatUrc>(&mut self) -> Option<Urc::Response> {
        let mut first = None;
        self.try_read_urc_with::<Urc, _>(|urc, _| {
            first = Some(urc);
            true
        });
        first
    }

    fn try_read_urc_with<Urc: AtatUrc, F: for<'b> FnOnce(Urc::Response, &'b [u8]) -> bool>(
        &mut self,
        handle: F,
    ) -> bool;

    fn max_urc_len() -> usize;
}

pub struct Client<
    'a,
    W: Write,
    Clk: Clock,
    Delay: DelayUs,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
> {
    writer: W,
    clock: &'a Clk,
    delay: Delay,
    res_reader: FrameConsumer<'a, RES_CAPACITY>,
    urc_reader: FrameConsumer<'a, URC_CAPACITY>,
    config: Config,
    cooldown_timer: Option<Timer<'a, OneShot, Running, Clk, Milliseconds>>,
}

impl<
        'a,
        W: Write,
        Clk: Clock,
        Delay: DelayUs,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
    > Client<'a, W, Clk, Delay, RES_CAPACITY, URC_CAPACITY>
{
    pub(crate) fn new(
        writer: W,
        clock: &'a Clk,
        delay: Delay,
        res_reader: FrameConsumer<'a, RES_CAPACITY>,
        urc_reader: FrameConsumer<'a, URC_CAPACITY>,
        config: Config,
    ) -> Self {
        Self {
            writer,
            clock,
            delay,
            res_reader,
            urc_reader,
            config,
            cooldown_timer: None,
        }
    }
}

impl<
        W: Write,
        Clk: Clock<T = u64>,
        Delay: DelayUs,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
    > Client<'_, W, Clk, Delay, RES_CAPACITY, URC_CAPACITY>
{
    fn start_cooldown_timer(&mut self) {
        self.cooldown_timer = Some(
            self.clock
                .new_timer(Milliseconds(self.config.cmd_cooldown))
                .into_oneshot()
                .start()
                .unwrap(),
        );
    }

    async fn wait_cooldown_timer(&mut self) {
        if let Some(cooldown) = self.cooldown_timer.take() {
            if let Ok(remaining) = cooldown.remaining() {
                self.delay.delay_ms(remaining.0).await.unwrap();
            }
        }
    }
}

impl<
        W: Write,
        Clk: Clock<T = u64>,
        Delay: DelayUs,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
    > AtatClient for Client<'_, W, Clk, Delay, RES_CAPACITY, URC_CAPACITY>
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
            return cmd.parse(Ok(&[])).map_err(|_| Error::Error);
        }

        let response = {
            let res_future = self.res_reader.read_async();
            pin_mut!(res_future);

            let timeout_future = self.delay.delay_ms(Cmd::MAX_TIMEOUT_MS);
            pin_mut!(timeout_future);

            match select(res_future, timeout_future).await {
                Either::Left((res, _)) => {
                    let mut grant = res.unwrap();
                    grant.auto_release(true);

                    let frame = Frame::decode(grant.as_ref());
                    let resp = match Response::from(frame) {
                        Response::Result(r) => r,
                        Response::Prompt(_) => Ok(&[][..]),
                    };

                    cmd.parse(resp)
                }
                Either::Right(_) => {
                    warn!("Received timeout after {}ms", Cmd::MAX_TIMEOUT_MS);
                    Err(Error::Timeout)
                }
            }
        };

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
