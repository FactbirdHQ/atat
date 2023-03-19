//! Serde serializer/deserializer for AT commands

#![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![allow(unstable_features)]
// Can't have unstable (incomplete) features + !warnings
// #![deny(warnings)]
#![allow(deprecated)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_const_for_fn)]
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![cfg_attr(feature = "hex_str_arrays", feature(generic_const_exprs))]

pub mod de;
pub mod ser;

pub use serde;

#[doc(inline)]
pub use self::de::{from_slice, from_str, hex_str::HexStr};
#[doc(inline)]
pub use self::ser::{to_string, to_vec, SerializeOptions};

#[allow(clippy::uninit_assumed_init)]
unsafe fn uninitialized<T>() -> T {
    core::mem::MaybeUninit::uninit().assume_init()
}
