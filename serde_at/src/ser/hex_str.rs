use crate::HexStr;
use core::fmt::{LowerHex, UpperHex, Write};
use core::ops::Deref;
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
        let mut string = heapless::String::<20>::new();
        let val = **self;
        match (self.add_0x_with_encoding, self.hex_in_caps) {
            (true, true) => write!(string, "0x{val:X}").unwrap(),
            (true, false) => write!(string, "0x{val:x}").unwrap(),
            (false, true) => write!(string, "{val:X}").unwrap(),
            (false, false) => write!(string, "{val:x}").unwrap(),
        }

        serializer.serialize_str(string.as_str())
    }
}
