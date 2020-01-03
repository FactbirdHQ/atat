use heapless::{
  consts::*,
  spsc::{Consumer, Producer},
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
    self.send_timeout(cmd, self.default_timeout)
  }

  fn send_timeout(&mut self, cmd: C, timeout: u32) -> Result<R, Error> {
    match self.cmd_p.enqueue(cmd) {
      Ok(_) => self.wait_responses(timeout),
      Err(_e) => Err(Error::Overflow),
    }
  }

  // Can these be made using rusts new shiny async/await?
  fn wait_responses(&mut self, timeout: u32) -> Result<R, Error> {
    // if timeout > 0 {
    // self.timer.start(timeout);
    loop {
      if let Some(result) = self.resp_c.dequeue() {
        // self.timer.stop();
        return result.map_err(|_e| Error::InvalidResponse);
      }
      if self.timer.wait().is_ok() {
        return Err(Error::Timeout);
      }
    }
    // }
    // Ok(R::None)
  }
}
