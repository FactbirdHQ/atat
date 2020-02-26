use heapless::{consts, spsc::Consumer, String};

use embedded_hal::serial;

use crate::error::{Error, NBResult, Result};
use crate::traits::{ATATCmd, ATATInterface, ATATUrc};
use crate::{Config, Mode};
use core::time::Duration;
use ticklock::timer::{Timer, TimerInstant};

type ResConsumer = Consumer<'static, Result<String<consts::U256>>, consts::U5, u8>;
type UrcConsumer = Consumer<'static, String<consts::U64>, consts::U10, u8>;

#[derive(Debug)]
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
