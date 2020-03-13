// #![cfg_attr(not(test), no_std)]
// #![feature(test)]

#[macro_use]
extern crate nb;
extern crate ufmt;
extern crate void;

mod client;
mod error;
mod ingress_manager;
mod traits;

pub use self::client::ATClient;
pub use self::error::Error;
pub use self::ingress_manager::IngressManager;
pub use self::traits::{ATATCmd, ATATInterface, ATATResp, ATATUrc};

#[cfg(feature = "derive")]
pub use atat_derive;

use embedded_hal::{serial, timer::CountDown};
use heapless::{consts, spsc::Queue, String};

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

/// Whether the AT client should block while waiting responses or return early.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Command {
    ClearBuffer,
    SetLineTerm(u8),
    SetFormat(u8),
    SetEcho(bool),
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Config {
    mode: Mode,
    line_term_char: u8,
    format_char: u8,
    at_echo_enabled: bool,
    cmd_cooldown: u32,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            mode: Mode::Blocking,
            line_term_char: '\r' as u8,
            format_char: '\n' as u8,
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

    pub fn with_line_term(mut self, c: u8) -> Self {
        self.line_term_char = c;
        self
    }

    pub fn with_format_char(mut self, c: u8) -> Self {
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
type ComQueue = Queue<Command, consts::U3, u8>;
type ClientParser<Tx, T> = (client::ATClient<Tx, T>, IngressManager);

pub fn new<Tx, T>(serial_tx: Tx, timer: T, config: Config) -> ClientParser<Tx, T>
where
    Tx: serial::Write<u8>,
    T: CountDown,
    T::Time: From<u32>,
{
    static mut RES_QUEUE: ResQueue = Queue(heapless::i::Queue::u8());
    static mut URC_QUEUE: UrcQueue = Queue(heapless::i::Queue::u8());
    static mut COM_QUEUE: ComQueue = Queue(heapless::i::Queue::u8());
    let (res_p, res_c) = unsafe { RES_QUEUE.split() };
    let (urc_p, urc_c) = unsafe { URC_QUEUE.split() };
    let (com_p, com_c) = unsafe { COM_QUEUE.split() };
    let parser = IngressManager::new(res_p, urc_p, com_c, &config);
    let client = ATClient::new(serial_tx, res_c, urc_c, com_p, timer, config);

    (client, parser)
}
