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

#[cfg(feature = "hex_str_arrays")]
mod unstable {

    use serde::{Serialize, Serializer};
    use crate::HexStr;
    use core::fmt::Write;

    impl <const N: usize> Serialize for HexStr<[u8; N]>
        where
            heapless::String::<{ (1 + N*4)*2 }>: Sized
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
        {
            let val: &[u8] = if self.skip_last_0_values {
                let mut index = 0;
                for i in (0..(N - 1)).rev() {
                    index = i;
                    if self.val[i] != 0 {
                        break;
                    }
                }
                &self.val[0..=index]
            } else {
                &self.val
            };

            let mut string = heapless::String::<{ (1 + N*4)*2 }>::new();
            let mut nibble_count = 0;
            if self.add_0x_with_encoding {
                string.push_str("0x").unwrap();
            }
            for byte in val.iter() {
                let mut byte_string = heapless::String::<4>::new();
                if self.hex_in_caps {
                    write!(byte_string, "{:02X}", *byte).unwrap();
                } else {
                    write!(byte_string, "{:02x}", *byte).unwrap();
                }
                if self.delimiter_after_nibble_count != 0 {
                    for v in byte_string.as_str().chars() {
                        if nibble_count != 0 && nibble_count % self.delimiter_after_nibble_count == 0 {
                            string.push(self.delimiter).unwrap();
                        }
                        nibble_count += 1;
                        string.push(v).unwrap();
                    }
                } else {
                    for v in byte_string.as_str().chars() {
                        string.push(v).unwrap();
                    }
                }
            }
            serializer.serialize_str(string.as_str())
        }
    }
}
