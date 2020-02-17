use heapless::{consts, spsc::Consumer, String};

use embedded_hal::{serial, timer::CountDown};

use crate::error::{Error, NBResult, Result};
use crate::traits::{ATATCmd, ATATInterface};
use crate::{Config, Mode};

// use dynstack::{DynStack, dyn_push};

use log::{info, error};

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
    // handlers: DynStack<dyn Fn(&str)>,
}

impl<Tx, T> ATClient<Tx, T>
where
    Tx: serial::Write<u8>,
    T: CountDown,
{
    pub fn new(tx: Tx, queue: ResConsumer, config: Config<T>) -> Self {
        Self {
            tx,
            res_c: queue,
            state: ClientState::Idle,
            mode: config.mode,
            // handlers: DynStack::<dyn Fn(&str)>::new(),
        }
    }
}
impl<Tx, T> ATClient<Tx, T>
where
    Tx: serial::Write<u8>,
    T: CountDown,
{
    pub fn register_urc_handler<F>(&mut self, handler: &'static F) -> core::result::Result<(), ()>
    where
        F: for<'a> Fn(&'a str)
    {
        // dyn_push!(self.handlers, handler);
        Ok(())
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

        let res = match self.mode {
            Mode::Blocking => block!(self.check_response(cmd)).map_err(nb::Error::Other)?,
            Mode::NonBlocking => self.check_response(cmd)?,
            Mode::Timeout(ref mut timer) => {
                timer.start(cmd.max_timeout_ms());
                block!(self.check_response(cmd)).map_err(nb::Error::Other)?
            }
        };

        match res {
            Some(r) => Ok(r),
            None => Err(nb::Error::WouldBlock),
        }
    }

    fn check_response<A: ATATCmd>(&mut self, cmd: &A) -> NBResult<Option<A::Response>> {
        if let Some(result) = self.res_c.dequeue() {
            return match result {
                Ok(resp) => {
                    if let ClientState::AwaitingResponse = self.state {
                        self.state = ClientState::Idle;
                        info!("{:?}\r", resp);
                        Ok(Some(cmd.parse(&resp).map_err(|e| {
                            error!("{:?}", e);
                            nb::Error::Other(e)
                        })?))
                    } else {
                        // URC
                        // for handler in self.handlers.iter() {
                        //     handler(&resp);
                        // };
                        Ok(None)
                    }
                }
                Err(e) => Err(nb::Error::Other(e)),
            };
        } else if let Mode::Timeout(ref mut timer) = self.mode {
            if timer.wait().is_ok() {
                self.state = ClientState::Idle;
                return Err(nb::Error::Other(Error::Timeout));
            }
        }
        Err(nb::Error::WouldBlock)
    }
}
