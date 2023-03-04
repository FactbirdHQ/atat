use crate::HexStr;
use core::fmt::{Write};
use core::ops::{ Deref };
use core::fmt::{LowerHex, UpperHex};
use serde::ser::Serialize;
use serde::Serializer;

impl<T> Serialize for HexStr<T>
where
    Self: Deref + Sized,
    <Self as Deref>::Target: Sized + UpperHex + LowerHex + Copy,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let val: <Self as Deref>::Target = **self;
        if self.delimiter_after_nibble_count > 0 {
            let mut string = heapless::String::<40>::new();
            let mut placeholder = heapless::String::<40>::new();
            if self.hex_in_caps {
                write!(string, "{val:X}").unwrap();
            } else {
                write!(string, "{val:x}").unwrap();
            }

            for (index, c) in string.chars().rev().enumerate() {
                if index != 0 && index % self.delimiter_after_nibble_count == 0 {
                    placeholder.push(self.delimiter).unwrap();
                }
                placeholder.push(c).unwrap();
            }

            string.clear();
            if self.add_0x_with_encoding {
                string.push('0').unwrap();
                string.push('x').unwrap();
            }
            for c in placeholder.chars().rev() {
                string.push(c).unwrap()
            }

            serializer.serialize_str(string.as_str())
        } else {
            let mut string = heapless::String::<20>::new();
            match (self.add_0x_with_encoding, self.hex_in_caps) {
                (true, true) => write!(string, "0x{val:X}").unwrap(),
                (true, false) => write!(string, "0x{val:x}").unwrap(),
                (false, true) => write!(string, "{val:X}").unwrap(),
                (false, false) => write!(string, "{val:x}").unwrap(),
            }
            serializer.serialize_str(string.as_str())
        }

    }
}
