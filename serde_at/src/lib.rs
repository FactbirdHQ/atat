// #![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::missing_errors_doc)]
#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

pub mod de;
pub mod ser;

pub use serde;

#[doc(inline)]
pub use self::de::{from_slice, CharVec, from_str};
#[doc(inline)]
pub use self::ser::{to_string, to_vec, Bytes, SerializeOptions};

unsafe fn uninitialized<T>() -> T {
    core::mem::MaybeUninit::uninit().assume_init()
}
