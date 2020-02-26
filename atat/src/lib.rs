#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate nb;
extern crate ticklock;
extern crate ufmt;

mod buffer;
mod client;
mod error;
mod parser;
mod traits;

pub use self::buffer::Buffer;
pub use self::client::ATClient;
pub use self::error::Error;
pub use self::parser::ATParser;
pub use self::traits::{ATATCmd, ATATInterface, ATATResp, ATATUrc};
#[cfg(feature = "derive")]
pub use atat_derive;

use embedded_hal::serial;
use heapless::{consts, spsc::Queue, String};
use ticklock::timer::Timer;

pub mod prelude {
    pub use crate::{ATATCmd, ATATInterface, ATATResp, ATATUrc};
}

/// Whether the AT client should block while waiting responses or return early.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Mode {
    /// The function call will wait as long as necessary to complete the operation
    Blocking,
    /// The function call will not wait at all to complete the operation, and only do what it can.
    NonBlocking,
    /// The function call will wait only up the max timeout of each command to complete the operation.
    Timeout,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Config {
    mode: Mode,
    line_term_char: char,
    format_char: char,
    at_echo_enabled: bool,
    cmd_cooldown: u32,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            mode: Mode::Blocking,
            line_term_char: '\r',
            format_char: '\n',
            at_echo_enabled: true,
            cmd_cooldown: 20,
        }
    }
}

impl Config {
    pub fn new(mode: Mode) -> Self {
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

    pub fn cmd_cooldown(mut self, ms: u32) -> Self {
        self.cmd_cooldown = ms;
        self
    }
}

type ResQueue = Queue<Result<String<consts::U256>, error::Error>, consts::U5, u8>;
type UrcQueue = Queue<String<consts::U64>, consts::U10, u8>;
type ClientParser<Rx, Tx, T> = (client::ATClient<Tx, T>, parser::ATParser<Rx>);

pub fn new<Rx, Tx, T>(serial: (Tx, Rx), timer: T, config: Config) -> ClientParser<Rx, Tx, T>
where
    Tx: serial::Write<u8>,
    Rx: serial::Read<u8>,
    T: Timer,
{
    static mut RES_QUEUE: ResQueue = Queue(heapless::i::Queue::u8());
    static mut URC_QUEUE: UrcQueue = Queue(heapless::i::Queue::u8());
    let (res_p, res_c) = unsafe { RES_QUEUE.split() };
    let (urc_p, urc_c) = unsafe { URC_QUEUE.split() };
    let parser = ATParser::new(serial.1, res_p, urc_p, &config);
    let client = client::ATClient::new(serial.0, res_c, urc_c, timer, config);

    (client, parser)
}
