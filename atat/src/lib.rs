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
//! [`atat_derive`]: <https://crates.io/crates/atat_derive>
//!
//! # Examples
//!
//! ### Command and response example without `atat_derive`:
//! ```
//! use atat::{AtatCmd, AtatResp, Error};
//! use core::fmt::Write;
//! use heapless::{consts, String, Vec};
//!
//! pub struct SetGreetingText<'a> {
//!     pub text: &'a str,
//! }
//!
//! pub struct GetGreetingText;
//!
//! pub struct NoResponse;
//!
//! impl AtatResp for NoResponse {};
//!
//! pub struct GreetingText {
//!     pub text: String<consts::U64>,
//! };
//!
//! impl AtatResp for GreetingText {};
//!
//! impl<'a> AtatCmd for SetGreetingText<'a> {
//!     type CommandLen = consts::U64;
//!     type Response = NoResponse;
//!
//!     fn as_bytes(&self) -> Vec<u8, Self::CommandLen> {
//!         let mut buf: Vec<u8, Self::CommandLen> = Vec::new();
//!         write!(buf, "AT+CSGT={}", self.text);
//!         buf
//!     }
//!
//!     fn parse(&self, resp: &[u8]) -> Result<Self::Response, Error> {
//!         Ok(NoResponse)
//!     }
//! }
//!
//! impl AtatCmd for GetGreetingText {
//!     type CommandLen = consts::U8;
//!     type Response = GreetingText;
//!
//!     fn as_bytes(&self) -> Vec<u8, Self::CommandLen> {
//!         Vec::from_slice(b"AT+CSGT?").unwrap()
//!     }
//!
//!     fn parse(&self, resp: &[u8]) -> Result<Self::Response, Error> {
//!         // Parse resp into `GreetingText`
//!         Ok(GreetingText {
//!             text: String::from(core::str::from_utf8(resp).unwrap()),
//!         })
//!     }
//! }
//! ```
//!
//! ### Same example with `atat_derive`:
//! ```
//! use atat::atat_derive::{AtatCmd, AtatResp};
//! use heapless::{consts, String};
//!
//! #[derive(Clone, AtatCmd)]
//! #[at_cmd("+CSGT", NoResponse)]
//! pub struct SetGreetingText<'a> {
//!     #[at_arg(position = 0, len = 32)]
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
//!     pub text: String<consts::U64>,
//! };
//! ```
//!
//! ### Basic usage example (More available in examples folder):
//! ```ignore
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
//! use atat::{atat_derive::{AtatResp, AtatCmd}};
//!
//! use heapless::{consts, spsc::Queue, String};
//!
//! use crate::rt::entry;
//! static mut INGRESS: Option<atat::IngressManager> = None;
//! static mut RX: Option<Rx<USART2>> = None;
//!
//!
//! #[derive(Clone, AtatResp)]
//! pub struct NoResponse;
//!
//! #[derive(Clone, AtatCmd)]
//! #[at_cmd("", NoResponse, timeout_ms = 1000)]
//! pub struct AT;
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
//!     static mut RES_QUEUE: ResQueue<consts::U256> = Queue(heapless::i::Queue::u8());
//!     static mut URC_QUEUE: UrcQueue<consts::U256, consts::U10> = Queue(heapless::i::Queue::u8());
//!     static mut COM_QUEUE: ComQueue = Queue(heapless::i::Queue::u8());
//!
//!     let queues = Queues {
//!         res_queue: unsafe { RES_QUEUE.split() },
//!         urc_queue: unsafe { URC_QUEUE.split() },
//!         com_queue: unsafe { COM_QUEUE.split() },
//!     };
//!
//!     let (tx, rx) = serial.split();
//!     let (mut client, ingress) =
//!         ClientBuilder::new(tx, timer, atat::Config::new(atat::Mode::Timeout)).build(queues);
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
//!         match client.send(&AT) {
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
//! - **`derive`** *(enabled by default)* - Re-exports [`atat_derive`] to allow
//!   deriving `Atat__` traits.
//! - **`defmt-default`** *(disabled by default)* - Enable log statements at
//!   INFO, or TRACE, level and up, to aid debugging. Powered by `defmt`.
//! - **`defmt-trace`** *(disabled by default)* - Enable log statements at TRACE
//!   level and up, to aid debugging. Powered by `defmt`.
//! - **`defmt-debug`** *(disabled by default)* - Enable log statements at DEBUG
//!   level and up, to aid debugging. Powered by `defmt`.
//! - **`defmt-info`** *(disabled by default)* - Enable log statements at INFO
//!   level and up, to aid debugging. Powered by `defmt`.
//! - **`defmt-warn`** *(disabled by default)* - Enable log statements at WARN
//!   level and up, to aid debugging. Powered by `defmt`.
//! - **`defmt-error`** *(disabled by default)* - Enable log statements at ERROR
//!   level and up, to aid debugging. Powered by `defmt`.

#![deny(warnings)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unused_unit)]
#![allow(clippy::use_self)]
#![allow(clippy::clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::used_underscore_binding)]
#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

mod builder;
mod client;
mod digest;
mod error;
pub mod helpers;
mod ingress_manager;
mod queues;
mod traits;
mod urc_matcher;

#[cfg(feature = "derive")]
pub use atat_derive;
#[cfg(feature = "derive")]
pub mod derive;

#[cfg(feature = "derive")]
pub use self::derive::AtatLen;

#[cfg(feature = "derive")]
pub use serde_at;

#[cfg(feature = "derive")]
pub use typenum;

#[cfg(feature = "derive")]
pub use heapless;

pub use builder::ClientBuilder;
pub use client::{Client, Mode};
pub use digest::{DefaultDigester, DigestResult, Digester};
pub use error::{Error, GenericError, InternalError};
pub use ingress_manager::IngressManager;
pub use queues::{ComQueue, Queues, ResQueue, UrcQueue};
pub use traits::{AtatClient, AtatCmd, AtatResp, AtatUrc};
pub use urc_matcher::{DefaultUrcMatcher, UrcMatcher, UrcMatcherResult};

/// Commands that can be sent from the client to the ingress manager, for
/// configuration after initial setup. This is also used for stuff like clearing
/// the receive buffer on command timeouts.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Command {
    /// Reset to initial state, including clearing the buffer,
    /// usually as a result of a command timeout
    Reset,
    /// Force the ingress manager into receive state
    ForceReceiveState,
}

/// Configuration of both the ingress manager, and the AT client. Some of these
/// parameters can be changed on the fly, through issuing a [`Command`] from the
/// client.
///
/// [`Command`]: enum.Command.html
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Config {
    mode: Mode,
    cmd_cooldown: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Blocking,
            cmd_cooldown: 20,
        }
    }
}

impl Config {
    #[must_use]
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn cmd_cooldown(mut self, ms: u32) -> Self {
        self.cmd_cooldown = ms;
        self
    }
}

#[cfg(test)]
mod tests {
    //! This module is required in order to satisfy the requirements of defmt, while running tests.
    //! Note that this will cause all log `defmt::` log statements to be thrown away.

    use core::ptr::NonNull;

    #[defmt::global_logger]
    struct Logger;
    impl defmt::Write for Logger {
        fn write(&mut self, _bytes: &[u8]) {}
    }

    unsafe impl defmt::Logger for Logger {
        fn acquire() -> Option<NonNull<dyn defmt::Write>> {
            Some(NonNull::from(&Logger as &dyn defmt::Write))
        }

        unsafe fn release(_: NonNull<dyn defmt::Write>) {}
    }

    defmt::timestamp!("");
}
