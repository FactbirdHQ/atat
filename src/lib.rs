#![feature(rustc_private)]
#![no_std]

#[macro_use]
extern crate nb;

mod buffer;
mod parser;
mod traits;
mod error;
pub mod utils;

pub type MaxCommandLen = heapless::consts::U60;
pub type MaxResponseLen = heapless::consts::U60;
pub type MaxResponseLines = heapless::consts::U8;

pub use self::buffer::Buffer;
pub use self::error::Error;
pub use self::parser::ATParser;
pub use self::traits::ATCommandInterface;

#[cfg(test)]
mod tests;
