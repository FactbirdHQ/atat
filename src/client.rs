use heapless::{
  ArrayLength,
  spsc::{Consumer, Producer},
};

use embedded_hal::timer::CountDown;

use crate::error::Error;
use crate::traits::ATInterface;

type CmdProducer<C, N> = Producer<'static, C, N, u8>;
type RespConsumer<R, N> = Consumer<'static, Result<R, Error>, N, u8>;

pub struct ATClient<T, C, R, CmdQueueLen, RespQueueLen>
where
  T: CountDown,
  CmdQueueLen: ArrayLength<C>,
  RespQueueLen: ArrayLength<Result<R, Error>>,
{
  cmd_p: CmdProducer<C, CmdQueueLen>,
  resp_c: RespConsumer<R, RespQueueLen>,
  default_timeout: T::Time,
  timer: T,
}

impl<T, C, R, CmdQueueLen, RespQueueLen> ATClient<T, C, R, CmdQueueLen, RespQueueLen>
where
  T: CountDown,
  T::Time: Copy,
  CmdQueueLen: ArrayLength<C>,
  RespQueueLen: ArrayLength<Result<R, Error>>,
{
  pub fn new(
    queues: (CmdProducer<C, CmdQueueLen>, RespConsumer<R, RespQueueLen>),
    default_timeout: T::Time,
    timer: T,
  ) -> Self {
    let (cmd_p, resp_c) = queues;
    Self {
      cmd_p,
      resp_c,
      default_timeout,
      timer,
    }
  }

  pub fn release(self) -> (CmdProducer<C, CmdQueueLen>, RespConsumer<R, RespQueueLen>) {
    (self.cmd_p, self.resp_c)
  }
}

impl<T, C, R, CmdQueueLen, RespQueueLen> ATInterface<T, C, R> for ATClient<T, C, R, CmdQueueLen, RespQueueLen>
where
  T: CountDown,
  T::Time: Copy,
  R: core::fmt::Debug,
  CmdQueueLen: ArrayLength<C>,
  RespQueueLen: ArrayLength<Result<R, Error>>,
{
  fn send(&mut self, cmd: C) -> Result<R, Error> {
    self.send_timeout(cmd, self.default_timeout)
  }

  fn send_timeout(&mut self, cmd: C, timeout: T::Time) -> Result<R, Error> {
    match self.cmd_p.enqueue(cmd) {
      Ok(_) => self.wait_response_timeout(timeout),
      Err(_e) => Err(Error::Overflow),
    }
  }

  fn wait_response_timeout(&mut self, timeout: T::Time) -> Result<R, Error> {
    self.timer.start(timeout);
    loop {
      if let Some(result) = self.resp_c.dequeue() {
        // self.timer.cancel().map_err(|_| Error::Timeout)?;
        return result.map_err(|_e| Error::InvalidResponse);
      }
      if self.timer.wait().is_ok() {
        return Err(Error::Timeout);
      }
    }
  }

  fn wait_response(&mut self) -> Result<R, Error> {
    self.wait_response_timeout(self.default_timeout)
  }

  fn peek_response(&mut self) -> &Result<R, Error> {
    self.timer.start(self.default_timeout);
    loop {
      if let Some(result) = self.resp_c.peek() {
        return result;
      }
      if self.timer.wait().is_ok() {
        return &Err(Error::Timeout);
      }
    }
  }
}
