use heapless::{consts, spsc::Consumer, String};

use embedded_hal::{serial, timer::CountDown};

use crate::error::{Error, NBResult, Result};
use crate::traits::{ATATCmd, ATATInterface};
use crate::Mode;

#[cfg(feature = "logging")]
use log::info;

type ResConsumer = Consumer<'static, Result<String<consts::U256>>, consts::U10, u8>;

#[derive(Debug)]
enum ClientState {
    Idle,
    AwaitingResponse,
}

pub struct ATClient<Tx, T>
where
    Tx: serial::Write<u8>,
    T: CountDown,
{
    tx: Tx,
    res_c: ResConsumer,
    // last_response_time: T::Time,
    state: ClientState,
    mode: Mode<T>,
}

impl<Tx, T> ATClient<Tx, T>
where
    Tx: serial::Write<u8>,
    T: CountDown,
{
    pub fn new(tx: Tx, queue: ResConsumer, mode: Mode<T>) -> Self {
        Self {
            tx,
            res_c: queue,
            state: ClientState::Idle,
            mode,
        }
    }
}

impl<Tx, T> ATATInterface for ATClient<Tx, T>
where
    Tx: serial::Write<u8>,
    T: CountDown,
    T::Time: From<u32>,
{
    fn send<A: ATATCmd>(&mut self, cmd: &A) -> NBResult<A::Response> {
        if let ClientState::Idle = self.state {
            for c in cmd.as_str().as_bytes() {
                block!(self.tx.write(*c)).ok();
            }
            block!(self.tx.flush()).ok();
            self.state = ClientState::AwaitingResponse;
        }

        match self.mode {
            Mode::Blocking => Ok(block!(self.check_response(cmd)).unwrap()),
            Mode::NonBlocking => self.check_response(cmd),
            Mode::Timeout(ref mut timer) => {
                timer.start(cmd.max_timeout_ms());
                Ok(block!(self.check_response(cmd)).unwrap())
            }
        }
    }

    fn check_response<A: ATATCmd>(&mut self, cmd: &A) -> NBResult<A::Response> {
        if let Some(result) = self.res_c.dequeue() {
            match result {
                Ok(resp) => {
                    if let ClientState::AwaitingResponse = self.state {
                        self.state = ClientState::Idle;
                        cmd.parse(&resp).map_err(nb::Error::Other)
                    } else {
                        // URC
                        Err(nb::Error::WouldBlock)
                    }
                }
                Err(e) => Err(nb::Error::Other(e)),
            }
        } else if let Mode::Timeout(ref mut timer) = self.mode {
            if timer.wait().is_ok() {
                self.state = ClientState::Idle;
                Err(nb::Error::Other(Error::Timeout))
            } else {
                Err(nb::Error::WouldBlock)
            }
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}
