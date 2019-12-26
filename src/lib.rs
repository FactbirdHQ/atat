#![feature(rustc_private)]
#![no_std]

#[macro_use]
extern crate nb;

mod buffer;
pub mod client;
mod error;
mod parser;
mod traits;
pub mod utils;

pub type MaxCommandLen = heapless::consts::U60;
pub type MaxResponseLen = heapless::consts::U60;
pub type MaxResponseLines = heapless::consts::U8;

pub use self::buffer::Buffer;
pub use self::error::Error;
pub use self::parser::ATParser;
pub use self::traits::{ATCommandInterface, ATInterface};

#[cfg(test)]
mod tests;

use embedded_hal::{serial, timer::CountDown};
use heapless::{consts, spsc::Queue};

type CmdQueue<C> = Queue<C, consts::U10, u8>;
type RespQueue<R> = Queue<Result<R, error::Error>, consts::U10, u8>;

pub fn new<Serial, C, R, T>(
  queues: (&'static mut CmdQueue<C>, &'static mut RespQueue<R>),
  serial: Serial,
  timer: T,
  default_timeout: u32,
) -> (client::ATClient<T, C, R>, parser::ATParser<Serial, C, R>)
where
  Serial: serial::Write<u8> + serial::Read<u8>,
  C: ATCommandInterface<R>,
  R: core::fmt::Debug,
  T: CountDown,
{
  let (wifi_cmd_p, wifi_cmd_c) = queues.0.split();
  let (wifi_resp_p, wifi_resp_c) = queues.1.split();

  let client = client::ATClient::new((wifi_cmd_p, wifi_resp_c), default_timeout, timer);
  let parser = ATParser::new(serial, (wifi_cmd_c, wifi_resp_p));

  (client, parser)
}
