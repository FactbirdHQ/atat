use heapless::{
  consts::*,
  spsc::{Consumer, Producer},
  Vec,
};

use embedded_hal::timer::CountDown;

use crate::error::Error;
use crate::traits::ATInterface;

type CmdProducer<C> = Producer<'static, C, U10, u8>;
type RespConsumer<R> = Consumer<'static, Result<R, Error>, U10, u8>;

pub struct ATClient<T, C, R> {
  cmd_p: CmdProducer<C>,
  resp_c: RespConsumer<R>,
  default_timeout: u32,
  timer: T,
}

impl<T, C, R> ATClient<T, C, R>
where
  T: CountDown,
{
  pub fn new(queues: (CmdProducer<C>, RespConsumer<R>), default_timeout: u32, timer: T) -> Self {
    let (cmd_p, resp_c) = queues;
    Self {
      cmd_p,
      resp_c,
      default_timeout,
      timer,
    }
  }

  pub fn release(self) -> (CmdProducer<C>, RespConsumer<R>) {
    (self.cmd_p, self.resp_c)
  }
}

impl<T, C, R> ATInterface<C, R> for ATClient<T, C, R>
where
  T: CountDown,
  R: core::fmt::Debug,
{
  fn send(&mut self, cmd: C) -> Result<R, Error> {
    self
      .send_multi_response(cmd)?
      .pop()
      .map_or_else(|| Err(Error::Write), Ok)
  }

  fn send_multi_response(&mut self, cmd: C) -> Result<Vec<R, U8>, Error> {
    self.send_multi_response_timeout(cmd, self.default_timeout)
  }

  fn send_multi_response_timeout(&mut self, cmd: C, timeout: u32) -> Result<Vec<R, U8>, Error> {
    match self.cmd_p.enqueue(cmd) {
      Ok(_) => self.wait_responses(timeout),
      Err(_e) => Err(Error::Overflow),
    }
  }

  fn send_timeout(&mut self, cmd: C, timeout: u32) -> Result<R, Error> {
    self
      .send_multi_response_timeout(cmd, timeout)?
      .pop()
      .map_or_else(|| Err(Error::Timeout), Ok)
  }

  fn send_no_response(&mut self, cmd: C) -> Result<(), Error> {
    self.send_multi_response_timeout(cmd, 0)?;
    Ok(())
  }

  // Can these be made using rusts new shiny async/await?
  fn wait_responses(&mut self, timeout: u32) -> Result<Vec<R, U8>, Error> {
    let mut responses = Vec::new();
    if timeout > 0 {
      // self.timer.start(timeout);
      loop {
        if let Some(result) = self.resp_c.dequeue() {
          match result {
            Ok(response) => {
              responses.push(response).unwrap();
              break;
            }
            Err(_e) => return Err(Error::InvalidResponse),
          }
        }
        if self.timer.wait().is_ok() {
          return Err(Error::Timeout);
        }
      }
    }
    Ok(responses)
  }
}
