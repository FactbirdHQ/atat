use serde::Serializer;
use crate::HexStr;
use serde::ser::{Serialize};
use core::fmt::{Write, UpperHex, LowerHex};
use std::ops::Deref;

impl <T> Serialize for HexStr<T>
    where
        HexStr<T>: Deref + Sized,
        <HexStr<T> as Deref>::Target: Sized + UpperHex + LowerHex + Copy
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        let mut string = heapless::String::<20>::new();
        let val = **self;
        if self.add_0x_with_encoding {
            if self.hex_in_caps {
                write!(string, "0x{:X}", val).unwrap();
            } else {
                write!(string, "0x{:x}", val).unwrap();
            }
        } else {
            if self.hex_in_caps {
                write!(string, "{:X}", val).unwrap();
            } else {
                write!(string, "{:x}", val).unwrap();
            }
        }

        serializer.serialize_str(string.as_str())
    }
}