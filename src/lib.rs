#![no_std]

#[macro_use]
extern crate nb;

mod buffer;
pub mod client;
mod error;
mod parser;
mod traits;
pub mod utils;

pub type MaxCommandLen = heapless::consts::U64;
pub type MaxResponseLen = heapless::consts::U64;
pub type MaxResponseLines = heapless::consts::U160;

pub use self::buffer::Buffer;
pub use self::error::Error;
pub use self::parser::ATParser;
pub use self::traits::{ATCommandInterface, ATInterface};

#[cfg(test)]
mod tests;

use embedded_hal::{serial, timer::CountDown};
use heapless::{spsc::Queue, ArrayLength};

type CmdQueue<C, N> = Queue<C, N, u8>;
type RespQueue<R, N> = Queue<Result<R, error::Error>, N, u8>;

pub fn new<Serial, C, R, T, RxBufferLen, CmdQueueLen, RespQueueLen>(
    queues: (
        &'static mut CmdQueue<C, CmdQueueLen>,
        &'static mut RespQueue<R, RespQueueLen>,
    ),
    serial: Serial,
    timer: T,
    default_timeout: T::Time,
) -> (
    client::ATClient<T, C, R, CmdQueueLen, RespQueueLen>,
    parser::ATParser<Serial, C, R, RxBufferLen, CmdQueueLen, RespQueueLen>,
)
where
    Serial: serial::Write<u8> + serial::Read<u8>,
    RxBufferLen: ArrayLength<u8>,
    CmdQueueLen: ArrayLength<C>,
    RespQueueLen: ArrayLength<Result<R, error::Error>>,
    C: ATCommandInterface<R>,
    R: core::fmt::Debug,
    T: CountDown,
    T::Time: Copy,
{
    let (wifi_cmd_p, wifi_cmd_c) = queues.0.split();
    let (wifi_resp_p, wifi_resp_c) = queues.1.split();

    let client = client::ATClient::new((wifi_cmd_p, wifi_resp_c), default_timeout, timer);
    let parser = ATParser::new(serial, (wifi_cmd_c, wifi_resp_p));

    (client, parser)
}
