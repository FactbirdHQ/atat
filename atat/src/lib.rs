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
//! use atat::{AtatCmd, AtatResp, Error, InternalError};
//! use core::fmt::Write;
//! use heapless::{String, Vec};
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
//!     pub text: String<64>,
//! };
//!
//! impl AtatResp for GreetingText {};
//!
//! impl<'a> AtatCmd for SetGreetingText<'a> {
//!     type Response = NoResponse;
//!
//!     fn write(&self, mut buf: &mut [u8]) -> usize {
//!         let buf_len = buf.len();
//!         use embedded_io::Write;
//!         write!(buf, "AT+CSGT={}", self.text);
//!         buf_len - buf.len()
//!     }
//!
//!     fn parse(&self, resp: Result<&[u8], InternalError>) -> Result<Self::Response, Error> {
//!         Ok(NoResponse)
//!     }
//! }
//!
//! impl AtatCmd for GetGreetingText {
//!     type Response = GreetingText;
//!
//!     fn write(&self, mut buf: &mut [u8]) -> usize {
//!         let cmd = b"AT+CSGT?";
//!         let len = cmd.len();
//!         buf[..len].copy_from_slice(cmd);
//!         len
//!     }
//!
//!     fn parse(&self, resp: Result<&[u8], InternalError>) -> Result<Self::Response, Error> {
//!         // Parse resp into `GreetingText`
//!         Ok(GreetingText {
//!             text: String::try_from(core::str::from_utf8(resp.unwrap()).unwrap()).unwrap(),
//!         })
//!     }
//! }
//! ```
//!
//! ### Same example with `atat_derive`:
//! ```
//! use atat::atat_derive::{AtatCmd, AtatResp};
//! use heapless::String;
//!
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
//!     pub text: String<64>,
//! };
//! ```
//!
//! ### Basic usage example (more available in the examples folder):
//!
//! The ingress and the client are decoupled. The ingress reads bytes from the
//! serial port and digests them into responses and URCs, while the client
//! writes commands and waits for the matching response. They communicate
//! through a shared [`ResponseSlot`] and [`UrcChannel`], so the ingress must be
//! driven concurrently with the client, typically as its own task.
//!
//! ```no_run
//! use atat::{
//!     asynch::{AtatClient, Client},
//!     atat_derive::{AtatCmd, AtatResp, AtatUrc},
//!     AtatIngress, Config, DefaultDigester, Ingress, ResponseSlot, UrcChannel,
//! };
//! use embedded_io_async::{Read, Write};
//!
//! #[derive(Clone, AtatResp)]
//! pub struct NoResponse;
//!
//! #[derive(Clone, AtatCmd)]
//! #[at_cmd("", NoResponse, timeout_ms = 1000)]
//! pub struct AT;
//!
//! #[derive(Clone, AtatResp)]
//! pub struct MessageWaitingIndication;
//!
//! #[derive(Clone, AtatUrc)]
//! pub enum Urc {
//!     #[at_urc("+UMWI")]
//!     MessageWaitingIndication(MessageWaitingIndication),
//! }
//!
//! const INGRESS_BUF_SIZE: usize = 1024;
//! const URC_CAPACITY: usize = 128;
//! const URC_SUBSCRIBERS: usize = 3;
//!
//! async fn run(serial_rx: impl Read, serial_tx: impl Write) {
//!     let res_slot = ResponseSlot::<INGRESS_BUF_SIZE>::new();
//!     let urc_channel = UrcChannel::<Urc, URC_CAPACITY, URC_SUBSCRIBERS>::new();
//!
//!     let mut ingress_buf = [0; INGRESS_BUF_SIZE];
//!     let mut ingress = Ingress::new(
//!         DefaultDigester::<Urc>::default(),
//!         &mut ingress_buf,
//!         &res_slot,
//!         &urc_channel,
//!     );
//!
//!     let mut cmd_buf = [0; 1024];
//!     let mut client = Client::new(serial_tx, &res_slot, &mut cmd_buf, Config::default());
//!     let mut urc_subscription = urc_channel.subscribe().unwrap();
//!
//!     embassy_futures::join::join(
//!         // Feeds the ingress from the serial port forever.
//!         // Spawn this as a separate task in a real application.
//!         ingress.read_from(serial_rx),
//!         async {
//!             let _response = client.send(&AT).await.unwrap();
//!             let _urc = urc_subscription.next_message_pure().await;
//!         },
//!     )
//!     .await;
//! }
//! ```
//!
//! # Optional Cargo Features
//!
//! - **`derive`** *(enabled by default)* - Re-exports [`atat_derive`], `serde_at`
//!   and `heapless` to allow deriving `Atat__` traits.
//! - **`bytes`** *(enabled by default)* - Re-exports `serde_bytes` and
//!   `heapless_bytes` to allow serializing and deserializing non-quoted byte
//!   slices correctly.
//! - **`std`** - Enables `std` on `serde_at`, `nom`, `embassy-time` and `embedded-io`.
//! - **`log`** - Log statements on various log levels, powered by `log`.
//! - **`defmt`** - Log statements on various log levels, powered by `defmt`.
//! - **`custom-error-messages`** - Adds an `Error::CustomMessage` variant
//!   carrying up to 64 bytes of the custom error text matched by
//!   `AtDigester::with_custom_error`.
//! - **`string_errors`** - Parses textual `+CME ERROR` / `+CMS ERROR` responses
//!   in addition to numeric codes.
//! - **`hex_str_arrays`** - Serializes hex strings to fixed-width byte arrays.
//!   Requires the nightly `generic_const_exprs` feature.
//! - **`heapless`** - Enables the `heapless` feature on `serde_at`.

// #![deny(warnings)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unused_unit)]
#![allow(clippy::use_self)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::type_complexity)]
#![allow(clippy::fallible_impl_from)]
#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![allow(async_fn_in_trait)]

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

mod config;
pub mod digest;
mod error;
pub mod helpers;
mod ingress;
mod response;
pub mod response_slot;
mod traits;
#[cfg(test)]
mod tx_mock;
pub mod urc_channel;
pub use nom;

pub mod asynch;
pub mod blocking;

#[cfg(feature = "bytes")]
pub use serde_bytes;

#[cfg(feature = "bytes")]
pub use heapless_bytes;

#[cfg(feature = "derive")]
pub use atat_derive;
#[cfg(feature = "derive")]
pub mod derive;

#[cfg(feature = "derive")]
pub use serde_at;

#[cfg(feature = "derive")]
pub use heapless;

pub use config::Config;
pub use digest::{AtDigester, AtDigester as DefaultDigester, DigestResult, Digester, Parser};
pub use error::{CmeError, CmsError, ConnectionError, Error, InternalError};
pub use ingress::{AtatIngress, Error as IngressError, Ingress};
pub use response::Response;
pub use response_slot::ResponseSlot;
pub use traits::{AtatCmd, AtatResp, AtatUrc};
pub use urc_channel::{UrcChannel, UrcSubscription};

#[cfg(test)]
#[cfg(feature = "defmt")]
mod tests {
    //! This module is required in order to satisfy the requirements of defmt, while running tests.
    //! Note that this will cause all log `defmt::` log statements to be thrown away.

    #[defmt::global_logger]
    struct Logger;

    unsafe impl defmt::Logger for Logger {
        fn acquire() {}

        unsafe fn flush() {}

        unsafe fn release() {}

        unsafe fn write(_bytes: &[u8]) {}
    }

    defmt::timestamp!("");

    #[export_name = "_defmt_panic"]
    fn panic() -> ! {
        panic!()
    }
}
