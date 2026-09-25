use super::AtatClient;
use crate::{helpers::LossyStr, AtatCmd, Config, DigestResult, Digester, Error, InternalError};
use embassy_time::{Duration, Instant};
use embedded_io::{Read, ReadReady, Write, WriteReady};

pub struct SimpleClient<'a, RW, D> {
    rw: RW,
    digester: D,
    buf: &'a mut [u8],
    pos: usize,
    config: Config,
    cooldown_timer: Option<Instant>,
}

impl<'a, RW: Read + Write + ReadReady + WriteReady, D: Digester> SimpleClient<'a, RW, D> {
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

    fn prepare_new_request(&mut self) -> Result<(), Error> {
        self.wait_cooldown_timer();
        Ok(())
    }

    fn flush_and_parse_response<Cmd: AtatCmd>(
        &mut self,
        cmd: &Cmd,
    ) -> Result<Cmd::Response, Error> {
        let until = Instant::now() + self.config.flush_timeout;
        wait_for_write(&mut self.rw, until)?;
        self.rw.flush().map_err(|_| Error::Write)?;

        self.start_cooldown_timer();

        if !Cmd::EXPECTS_RESPONSE_CODE {
            return cmd.parse(Ok(&[]));
        }

        self.pos = 0;

        let timeout = Duration::from_millis(Cmd::MAX_TIMEOUT_MS.into());
        let until = Instant::now() + timeout;
        loop {
            self.read_response_chunk(until)?;
            while self.pos > 0 {
                match self.digest() {
                    (Some(resp), _) => return cmd.parse(resp),
                    (_, 0) => break,
                    (_, swallowed) => self.consume(swallowed),
                }
            }
        }
    }

    fn read_response_chunk(&mut self, until: Instant) -> Result<(), Error> {
        wait_for_read(&mut self.rw, until)?;
        self.pos += self
            .rw
            .read(&mut self.buf[self.pos..])
            .map_err(|_| Error::Read)?;

        trace!(
            "Buffer contents: ({:?} bytes) '{:?}'",
            self.pos,
            LossyStr(&self.buf[..self.pos])
        );

        Ok(())
    }

    fn digest(&mut self) -> (Option<Result<&[u8], InternalError<'_>>>, usize) {
        let (result, swallowed) = self.digester.digest(&self.buf[..self.pos]);
        match &result {
            DigestResult::None if swallowed > 0 => debug!(
                "Received echo or whitespace ({}/{}): {:?}",
                swallowed,
                self.pos,
                LossyStr(&self.buf[..swallowed])
            ),
            DigestResult::None => {}
            DigestResult::Urc(urc_line) => {
                warn!("Unable to handle URC! Ignoring: {:?}", LossyStr(urc_line))
            }
            DigestResult::Prompt(_) => {
                debug!("Received prompt ({}/{})", swallowed, self.pos);
            }
            DigestResult::Response(Ok([])) => debug!("Received OK ({}/{})", swallowed, self.pos),
            DigestResult::Response(Ok(r)) => debug!(
                "Received response ({}/{}): {:?}",
                swallowed,
                self.pos,
                LossyStr(r)
            ),
            DigestResult::Response(Err(e)) => warn!(
                "Received error response ({}/{}): {:?}",
                swallowed, self.pos, e
            ),
        }
        let result = match result {
            DigestResult::Prompt(_) => Some(Ok(&[][..])),
            DigestResult::Response(resp) => Some(resp),
            _ => None,
        };
        (result, swallowed)
    }

    fn consume(&mut self, amt: usize) {
        self.buf.copy_within(amt..self.pos, 0);
        self.pos -= amt;
    }

    fn start_cooldown_timer(&mut self) {
        self.cooldown_timer = Some(Instant::now() + self.config.cmd_cooldown);
    }

    fn wait_cooldown_timer(&mut self) {
        if let Some(cooldown) = self.cooldown_timer.take() {
            while Instant::now() < cooldown {
                core::hint::spin_loop();
            }
        }
    }
}

impl<RW: Read + ReadReady + Write + WriteReady, D: Digester> AtatClient
    for SimpleClient<'_, RW, D>
{
    type Writer = RW;

    fn inner(&mut self) -> &mut RW {
        &mut self.rw
    }

    fn send_with<Cmd: AtatCmd>(
        &mut self,
        cmd: &Cmd,
        write: impl FnOnce(&mut RW) -> Result<(), RW::Error>,
    ) -> Result<Cmd::Response, Error> {
        self.prepare_new_request()?;

        write(&mut self.rw).map_err(|_| Error::Write)?;

        self.flush_and_parse_response(cmd)
    }

    fn send<Cmd: AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, Error> {
        let len = cmd.write(self.buf);

        if len < 50 {
            debug!("Sending command: {:?}", LossyStr(&self.buf[..len]));
        } else {
            debug!("Sending command with long payload ({} bytes)", len);
        }

        self.prepare_new_request()?;

        let until = Instant::now() + self.config.tx_timeout;
        let mut at = 0;
        while at < len {
            wait_for_write(&mut self.rw, until)?;
            at += self
                .rw
                .write(&self.buf[at..len])
                .map_err(|_| Error::Write)?;
        }

        self.flush_and_parse_response(cmd)
    }
}

fn wait_for_write(w: &mut impl WriteReady, until: Instant) -> Result<(), Error> {
    while Instant::now() < until {
        if w.write_ready().map_err(|_| Error::Write)? {
            return Ok(());
        }
    }
    Err(Error::Timeout)
}

fn wait_for_read(r: &mut impl ReadReady, until: Instant) -> Result<(), Error> {
    while Instant::now() < until {
        if r.read_ready().map_err(|_| Error::Read)? {
            return Ok(());
        }
    }
    Err(Error::Timeout)
}
