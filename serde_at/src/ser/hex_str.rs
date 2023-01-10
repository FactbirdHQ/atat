use crate::HexStr;
use core::fmt::{LowerHex, UpperHex, Write};
use core::ops::Deref;
use serde::ser::Serialize;
use serde::Serializer;

impl<T> Serialize for HexStr<T>
where
    HexStr<T>: Deref + Sized,
    <HexStr<T> as Deref>::Target: Sized + UpperHex + LowerHex + Copy,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut string = heapless::String::<20>::new();
        let val = **self;
        if self.add_0x_with_encoding {
            if self.hex_in_caps {
                write!(string, "0x{val:X}").unwrap();
            } else {
                write!(string, "0x{val:x}").unwrap();
            }
        } else if self.hex_in_caps {
            write!(string, "{val:X}").unwrap();
        } else {
            write!(string, "{val:x}").unwrap();
        }

        serializer.serialize_str(string.as_str())
    }
}
