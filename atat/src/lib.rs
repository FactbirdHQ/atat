#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate nb;
extern crate ufmt;

mod buffer;
pub mod client;
mod error;
mod parser;
mod traits;

pub use self::buffer::Buffer;
pub use self::error::Error;
pub use self::parser::ATParser;
pub use self::traits::{ATATCmd, ATATInterface, ATATResp};
#[cfg(feature = "derive")]
pub use atat_derive;

use embedded_hal::{serial, timer::CountDown};
use heapless::{consts, spsc::Queue, ArrayLength, String};

/// Whether the AT client should block while waiting responses or return early.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Mode<T>
where
    T: CountDown,
{
    /// The function call will wait as long as necessary to complete the operation
    Blocking,
    /// The function call will not wait at all to complete the operation, and only do what it can.
    NonBlocking,
    /// The function call will wait only up the max timeout of each command to complete the operation.
    Timeout(T),
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Config<T>
where
    T: CountDown,
{
    mode: Mode<T>,
    line_term_char: char,
    format_char: char,
    at_echo_enabled: bool,
}

impl<T> Default for Config<T>
where
    T: CountDown,
{
    fn default() -> Config<T> {
        Config {
            mode: Mode::Blocking,
            line_term_char: '\r',
            format_char: '\n',
            at_echo_enabled: true,
        }
    }
}

impl<T> Config<T>
where
    T: CountDown,
{
    pub fn new(mode: Mode<T>) -> Self {
        Config {
            mode,
            ..Config::default()
        }
    }

    pub fn with_line_term(mut self, c: char) -> Self {
        self.line_term_char = c;
        self
    }

    pub fn with_format_char(mut self, c: char) -> Self {
        self.format_char = c;
        self
    }

    pub fn with_at_echo(mut self, e: bool) -> Self {
        self.at_echo_enabled = e;
        self
    }
}

type ResQueue = Queue<Result<String<consts::U256>, error::Error>, consts::U10, u8>;
type ClientParser<Rx, Tx, T, RxBufferLen> = (
    client::ATClient<Tx, T>,
    parser::ATParser<Rx, RxBufferLen>,
);

pub fn new<Rx, Tx, RxBufferLen, T>(
    queue: &'static mut ResQueue,
    serial: (Tx, Rx),
    config: Config<T>,
) -> ClientParser<Rx, Tx, T, RxBufferLen>
where
    Tx: serial::Write<u8>,
    Rx: serial::Read<u8>,
    RxBufferLen: ArrayLength<u8>,
    T: CountDown,
{
    let (res_p, res_c) = queue.split();
    let parser = ATParser::new(serial.1, res_p, &config);
    let client = client::ATClient::new(serial.0, res_c, config);

    (client, parser)
}
