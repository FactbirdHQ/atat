//! A helper crate to abstract away the state management and string parsing of
//! AT command communication.
//!
//! It works by creating structs for each AT command, that each implements
//! [`AtatCmd`]. With corresponding response structs that each implements
//! [`AtatResp`].
//!
//! This can be simplified alot using the [`atat_derive`] crate!
//!
//! [`AtatCmd`]: trait.AtatCmd.html
//! [`AtatResp`]: trait.AtatResp.html
//! [`atat_derive`]: https://crates.io/crates/atat_derive
//!
//! # Examples
//!
//! ### Command and response example without atat_derive:
//! ```
//! pub struct SetGreetingText<'a> {
//!     pub text: &'a str,
//! }
//!
//! pub struct GetGreetingText;
//!
//! pub struct NoResponse;
//!
//! pub struct GreetingText {
//!     pub text: String<consts::U64>
//! };
//!
//! impl<'a> AtatCmd for SetGreetingText<'a> {
//!     type CommandLen = consts::U64;
//!     type Response = NoResponse;
//!
//!     fn as_str(&self) -> String<Self::CommandLen> {
//!         let buf: String<Self::CommandLen> = String::new();
//!         write!(buf, "AT+CSGT={}", self.text);
//!         buf
//!     }
//!
//!     fn parse(&self, resp: &str) -> Result<Self::Response> {
//!         NoResponse
//!     }
//! }
//!
//! impl AtatCmd for GetGreetingText<'a> {
//!     type CommandLen = consts::U8;
//!     type Response = GreetingText;
//!
//!     fn as_str(&self) -> String<Self::CommandLen> {
//!         String::from("AT+CSGT?")
//!     }
//!
//!     fn parse(&self, resp: &str) -> Result<Self::Response> {
//!         // Parse resp into `GreetingText`
//!         GreetingText { text: String::from(resp) }
//!     }
//! }
//! ```
//!
//! ### Same example with atat_derive:
//! ```
//! #[derive(Clone, AtatCmd)]
//! #[at_cmd("+CSGT", NoResponse)]
//! pub struct SetGreetingText<'a> {
//!     #[at_arg(position = 0)]
//!     pub text: &'a str,
//! }
//!
//! #[derive(Clone, AtatCmd)]
//! #[at_cmd("+CSGT?", GreetingText)]
//! pub struct GetGreetingText;
//!
//! #[derive(Clone, AtatResp)]
//! pub struct NoResponse;
//!
//! #[derive(Clone, AtatResp)]
//! pub struct GreetingText {
//!     #[at_arg(position = 0)]
//!     pub text: String<consts::U64>
//! };
//!
//! ```
//!
//! ### Basic usage example (More available in examples folder):
//! ```
//! mod common;
//!
//! use cortex_m::asm;
//! use hal::{
//!     gpio::{
//!         gpioa::{PA2, PA3},
//!         Alternate, Floating, Input, AF7,
//!     },
//!     pac::{interrupt, Peripherals, USART2},
//!     prelude::*,
//!     serial::{Config, Event::Rxne, Rx, Serial},
//!     timer::{Event, Timer},
//! };
//!
//! use atat::{driver, prelude::*};
//!
//! use heapless::{consts, spsc::Queue, String};
//!
//! use crate::rt::entry;
//! static mut INGRESS: Option<atat::IngressManager> = None;
//! static mut RX: Option<Rx<USART2>> = None;
//!
//! #[entry]
//! fn main() -> ! {
//!     let p = Peripherals::take().unwrap();
//!
//!     let mut flash = p.FLASH.constrain();
//!     let mut rcc = p.RCC.constrain();
//!     let mut pwr = p.PWR.constrain(&mut rcc.apb1r1);
//!
//!     let mut gpioa = p.GPIOA.split(&mut rcc.ahb2);
//!
//!     let clocks = rcc.cfgr.freeze(&mut flash.acr, &mut pwr);
//!
//!     let tx = gpioa.pa2.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
//!     let rx = gpioa.pa3.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
//!
//!     let mut timer = Timer::tim7(p.TIM7, 1.hz(), clocks, &mut rcc.apb1r1);
//!     let at_timer = Timer::tim6(p.TIM6, 100.hz(), clocks, &mut rcc.apb1r1);
//!
//!     let mut serial = Serial::usart2(
//!         p.USART2,
//!         (tx, rx),
//!         Config::default().baudrate(115_200.bps()),
//!         clocks,
//!         &mut rcc.apb1r1,
//!     );
//!
//!     serial.listen(Rxne);
//!
//!     let (tx, rx) = serial.split();
//!     let (mut client, ingress) = driver!(tx, at_timer, atat::Config::new(atat::Mode::Timeout));
//!
//!     unsafe { INGRESS = Some(ingress) };
//!     unsafe { RX = Some(rx) };
//!
//!     // configure NVIC interrupts
//!     unsafe { cortex_m::peripheral::NVIC::unmask(hal::stm32::Interrupt::TIM7) };
//!     timer.listen(Event::TimeOut);
//!
//!     // if all goes well you should reach this breakpoint
//!     asm::bkpt();
//!
//!     loop {
//!         asm::wfi();
//!
//!         match client.send(&common::AT) {
//!             Ok(response) => {
//!                 // Do something with response here
//!             }
//!             Err(e) => {}
//!         }
//!     }
//! }
//!
//! #[interrupt]
//! fn TIM7() {
//!     let ingress = unsafe { INGRESS.as_mut().unwrap() };
//!     ingress.parse_at();
//! }
//!
//! #[interrupt]
//! fn USART2() {
//!     let ingress = unsafe { INGRESS.as_mut().unwrap() };
//!     let rx = unsafe { RX.as_mut().unwrap() };
//!     if let Ok(d) = nb::block!(rx.read()) {
//!         ingress.write(&[d]);
//!     }
//! }
//! ```
//! # Optional Cargo Features
//!
//! - **`derive`** *(enabled by default)* — Enables and re-exports
//!   [`atat_derive`].
//! - **`logging`** *(disabled by default)* — Prints useful logging information,
//!   including incoming and outgoing bytes on the `TRACE` level.

#![cfg_attr(not(test), no_std)]

mod client;
mod error;
mod ingress_manager;
mod queues;
mod traits;

#[cfg(feature = "derive")]
pub use atat_derive;

#[cfg(feature = "derive")]
pub mod derive;

#[cfg(feature = "derive")]
pub use self::derive::AtatLen;

pub use self::client::Client;
pub use self::error::Error;
pub use self::ingress_manager::{
    get_line, IngressManager, NoopUrcMatcher, UrcMatcher, UrcMatcherResult,
};
pub use self::queues::{ComQueue, ResQueue, UrcQueue};
pub use self::traits::{AtatClient, AtatCmd, AtatResp, AtatUrc};

pub mod prelude {
    //! The prelude is a collection of all the traits in this crate
    //!
    //! The traits have been renamed to avoid collisions with other items when
    //! performing a glob import.
    pub use crate::AtatClient as _atat_AtatClient;
    pub use crate::AtatCmd as _atat_AtatCmd;
    pub use crate::AtatResp as _atat_AtatResp;
    pub use crate::AtatUrc as _atat_AtatUrc;

    #[cfg(feature = "derive")]
    pub use crate::AtatLen as _atat_AtatLen;
}

#[cfg(feature = "logging")]
#[macro_export]
macro_rules! log_str {
    ($level:ident, $fmt:expr, $buf:expr) => {
        match core::str::from_utf8(&$buf) {
            Ok(s) => log::$level!($fmt, s),
            Err(_) => log::$level!($fmt, $buf),
        }
    };
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

/// Commands that can be sent from the client to the ingress manager, for
/// configuration after initial setup. This is also used for stuff like clearing
/// the receive buffer on command timeouts.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Command {
    /// Clear the rx buffer, usually as a result of a command timeout
    ClearBuffer,
    /// Force the ingress manager into the given state
    ForceState(ingress_manager::State),
    /// Change the line termination character, must be called af setting `ATS3=`
    SetLineTerm(u8),
    /// Change the format character, must be called af setting `ATS4=`
    SetFormat(u8),
    /// Enable or disable AT echo, must be called after setting `ATE`
    SetEcho(bool),
}

/// Configuration of both the ingress manager, and the AT client. Some of these
/// parameters can be changed on the fly, through issuing a [`Command`] from the
/// client.
///
/// [`Command`]: enum.Command.html
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
            line_term_char: b'\r',
            format_char: b'\n',
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

/// Macro to ease the setup of queues, [`Client`] and [`IngressManager`]
///
#[macro_export]
macro_rules! driver {
    ($tx:expr, $timer:expr, $config:expr) => {
        driver!($tx, $timer, $config, None)
    };
    ($tx:expr, $timer:expr, $config:expr, $matcher:expr) => {
        driver!($tx, $timer, $config, None, consts::U256)
    };
    ($tx:expr, $timer:expr, $config:expr, $matcher:expr, $rxbuflen:ty) => {
        driver!(
            $tx,
            $timer,
            $config,
            None,
            $rxbuflen,
            consts::U3,
            consts::U5,
            consts::U10
        )
    };
    ($tx:expr, $timer:expr, $config:expr, $matcher:expr, $rxbuflen:ty, $comcap:ty, $rescap:ty, $urccap:ty) => {{
        static mut RES_QUEUE: atat::ResQueue<$rxbuflen, $rescap> =
            heapless::spsc::Queue(heapless::i::Queue::u8());
        static mut URC_QUEUE: atat::UrcQueue<$rxbuflen, $urccap> =
            heapless::spsc::Queue(heapless::i::Queue::u8());
        static mut COM_QUEUE: atat::ComQueue<$comcap> =
            heapless::spsc::Queue(heapless::i::Queue::u8());

        let (res_p, res_c) = unsafe { RES_QUEUE.split() };
        let (urc_p, urc_c) = unsafe { URC_QUEUE.split() };
        let (com_p, com_c) = unsafe { COM_QUEUE.split() };

        let parser =
            ::atat::IngressManager::with_custom_urc_matcher(res_p, urc_p, com_c, $config, $matcher);
        let client = ::atat::Client::new($tx, res_c, urc_c, com_p, $timer, $config);

        (client, parser)
    }};
}
