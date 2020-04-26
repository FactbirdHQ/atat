//! Derive crate for ATAT
//!
//! This crate provides derive macros for automatically deriving
//! [`atat::AtatCmd`], [`atat::AtatResp`], [`atat::AtatUrc`], [`atat::AtatEnum`]
//! and [`atat::AtatLen`]
//!
//! [`atat::AtatCmd`]: ../atat/trait.AtatCmd.html
//! [`atat::AtatResp`]: ../atat/trait.AtatResp.html
//! [`atat::AtatUrc`]: ../atat/trait.AtatUrc.html
//! [`atat::AtatEnum`]: ../atat/trait.AtatEnum.html
//! [`atat::AtatLen`]: ../atat/derive/trait.AtatLen.html
//!
//! # Examples
//!
//! ### AtatCmd
//! See [`AtatCmd`] for descriptions and documentation on required and allowed
//! attributes
//!
//! [`AtatCmd`]: derive.AtatCmd.html
//!
//! ```
//! // Serializing the following struct, results in `AT+USORD=<socket>,<length>\r\n`
//! #[derive(AtatCmd)]
//! #[at_cmd("+USORD", SocketData)]
//! pub struct ReadSocketData {
//!     #[at_arg(position = 0)]
//!     pub socket: u8,
//!     #[at_arg(position = 1)]
//!     pub length: usize,
//! }
//! ```

extern crate proc_macro;
extern crate proc_macro2;

mod cmd;
mod enum_;
mod len;
mod parse;
mod resp;
mod urc;

use crate::proc_macro::TokenStream;

/// Automatically derive [`atat::AtatResp`] trait
///
/// [`atat::AtatResp`]: ../atat/trait.AtatResp.html
///
#[proc_macro_derive(AtatResp, attributes(at_arg))]
pub fn derive_atat_resp(input: TokenStream) -> TokenStream {
    resp::atat_resp(input)
}

/// Automatically derive [`atat::AtatUrc`] trait
///
/// [`atat::AtatUrc`]: ../atat/trait.AtatUrc.html
///
#[proc_macro_derive(AtatUrc, attributes(at_urc))]
pub fn derive_atat_urc(input: TokenStream) -> TokenStream {
    urc::atat_urc(input)
}

/// Automatically derive [`atat::AtatEnum`] trait
///
/// [`atat::AtatEnum`]: ../atat/trait.AtatEnum.html
/// [`atat::AtatLen`]: ../atat/trait.AtatLen.html
///
/// This trait implementation is equivalent to using
/// [serde_repr](https://docs.rs/serde_repr/0.1.5/serde_repr/), thus removing
/// the need for this package in the Atat context.
///
/// Furthermore it automatically implements [`atat::AtatLen`], based on the data
/// type given in the container attribute.
///
/// ### Container attribute (`#[at_enum(..)]`)
/// The AtatEnum derive macro comes with an option of annotating the struct with
/// a container attribute `#[at_enum(..)]`.
///
/// The container attribute only allows specifying a single parameter, that is
/// non-optional if the container attribute is present. The parameter allows
/// describing the underlying data type of the enum, and thus the maximum
/// allowed value of the fields. Only integer types are allowed (`u8`, `u16`, `u32`,
/// `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128`, `usize`, `isize`). Eg. `#[at_enum(u16)]`.
///
/// **Note**: `at_enum` defaults to `u8`
///
/// ### Field attribute (`#[at_arg(..)]`)
/// The AtatEnum derive macro comes with an optional field attribute
/// `#[at_arg(..)]`, that can be specified o some or all of the fields.
///
/// Allowed options for `at_arg` are:
/// - `value` **integer** The value of the serialized field
///
#[proc_macro_derive(AtatEnum, attributes(at_enum, at_arg))]
pub fn derive_atat_enum(input: TokenStream) -> TokenStream {
    enum_::atat_enum(input)
}

/// Automatically derive [`atat::AtatCmd`] trait
///
/// [`atat::AtatCmd`]: ../atat/trait.AtatCmd.html
///
///
/// ### Container attribute (`#[at_cmd(..)]`)
/// The AtatCmd derive macro comes with a requirement of annotating the struct
/// with a container attribute `#[at_cmd(..)]`.
///
/// This container attribute requires specifying at least a command and an
/// expected response struct as: `#[at_cmd("+USORD", SocketData)]` where
/// `SocketData` is any type implementing `AtatResp`.
///
/// Furthermore the container attribute allows specifying some additional
/// options to tweak the command. All optional attributes takes the form `<key>
/// = <value>`, eg. `#[at_cmd("+USORD", SocketData, timeout_ms = 10000)]`
///
/// Allowed options are:
/// - `timeout_ms`: **integer** The maximum timeout in milliseconds of the
///   command
/// - `abortable`: **bool** Whether or not the command can be aborted
/// - `force_receive_state`: **bool** Force the ingress manager into receive
///   state immediately after sending (don't wait for echo). This is useful in
///   some command patterns.
/// - `value_sep`: **bool** Disable the seperator between the command and any
///   parameters (default true). Useful to create "fixed" commands, eg.
///   `#[at_cmd("+UDCONF=1", NoResponse, value_sep = false)]`.
/// - `cmd_prefix`: **string** Overwrite the prefix of the command (default
///   'AT'). Can also be set to '' (empty).
/// - `termination`: **string** Overwrite the line termination of the command
///   (default '\r\n'). Can also be set to '' (empty).
///
/// ### Field attribute (`#[at_arg(..)]`)
/// The AtatCmd derive macro comes with an optional field attribute
/// `#[at_arg(..)]`, that can be specified on some or all of the fields.
///
/// Allowed options for `at_arg` are:
/// - position: **integer** The index of the field in the resulting command
///   string. (eg. for command `AT+CMD=a,b`, field `a` would have `position = 1`
///   and field `b` would have `position = 2`) (defaults to order of the fields
///   in the struct)
///
#[proc_macro_derive(AtatCmd, attributes(at_cmd, at_arg))]
pub fn derive_atat_cmd(input: TokenStream) -> TokenStream {
    cmd::atat_cmd(input)
}

/// Automatically derive [`atat::AtatLen`] trait
///
/// [`atat::AtatLen`]: ../atat/derive/trait.AtatLen.html
///
/// This requires all of the fields to also implement [`atat::AtatLen`]
#[proc_macro_derive(AtatLen, attributes(at_arg))]
pub fn derive_atat_len(input: TokenStream) -> TokenStream {
    len::atat_len(input)
}
