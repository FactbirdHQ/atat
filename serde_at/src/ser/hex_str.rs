use crate::HexStr;
use core::fmt::Write;
use serde::ser::Serialize;
use serde::Serializer;

macro_rules! impl_hex_str_serialize {
    ($type:ty, $len:expr, $len_delimited:expr) => {
        impl Serialize for HexStr<$type> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let val: $type = **self;
                if self.delimiter_after_nibble_count > 0 {
                    let mut string = heapless::String::<$len_delimited>::new();
                    let mut placeholder = heapless::String::<$len_delimited>::new();
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
                        string.push(c).unwrap();
                    }

                    serializer.serialize_str(string.as_str())
                } else {
                    let mut string = heapless::String::<$len>::new();
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
    };
}

impl_hex_str_serialize!(u8, 8, 10);
impl_hex_str_serialize!(u16, 12, 18);
impl_hex_str_serialize!(u32, 20, 30);
impl_hex_str_serialize!(u64, 36, 66);
impl_hex_str_serialize!(u128, 68, 130);
