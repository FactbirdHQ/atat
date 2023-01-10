use core::fmt;
use core::marker::PhantomData;
use core::ops::{Deref, Shl};
use serde::de::Visitor;
use serde::*;

struct HexLiteralVisitor<T> {
    _ty: PhantomData<T>,
}

/// HexStr<T>
/// A hex string. Has phantom data used in serializing whether to add a 0x to the encoding
/// and to make the hex value in capital letters or not.
#[derive(Clone, PartialEq)]
pub struct HexStr<T> {
    /// Value of the hex string. Can be dereferenced
    pub val: T,
    /// Flag to add 0x when serializing the value
    pub add_0x_with_encoding: bool,
    /// Flag to serialize the hex in capital letters
    pub hex_in_caps: bool,
}

impl<T> Default for HexStr<T>
where
    T: Default,
{
    fn default() -> Self {
        HexStr {
            val: T::default(),
            add_0x_with_encoding: false,
            hex_in_caps: false,
        }
    }
}

macro_rules! impl_hex_literal_visitor {
    ($($int_type:ty)*) => {$(
        impl<'de> Visitor<'de> for HexLiteralVisitor<$int_type> {
            type Value = $int_type;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an unsigned integer in hexadecimal notation")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let mut s = core::str::from_utf8(v)
                    .map_err(serde::de::Error::custom)?;
                if s.starts_with("0x") || s.starts_with("0X") {
                    s = &s[2..];
                }

                let mut ret: $int_type = 0;

                for c in s.chars() {
                    let v = match c {
                        '0'..='9' => (c as $int_type) - ('0' as $int_type),
                        'A'..='F' => 0xa + ((c as $int_type) - ('A' as $int_type)),
                        'a'..='f' => 0xa + ((c as $int_type) - ('a' as $int_type)),
                        _ => 0
                    };

                    ret = ret
                        .shl(4i32)
                        .checked_add(v)
                        .ok_or(serde::de::Error::custom("Invalid number"))?;
                }

                Ok(ret)
            }
        }

        impl<'de> Deserialize<'de> for HexStr<$int_type> {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
            {
                let val = deserializer.deserialize_bytes(HexLiteralVisitor::<$int_type> { _ty: PhantomData })?;
                Ok(HexStr { val, ..Default::default() })
            }
        }

        impl Deref for HexStr<$int_type> {
            type Target = $int_type;

            fn deref(&self) -> &Self::Target {
                &self.val
            }
        }
    )*}
}

impl_hex_literal_visitor! { u8 u16 u32 u64 u128 }

#[cfg(test)]
mod tests {
    use crate::de::hex_str::HexStr;

    #[test]
    pub fn test_parsing_a_hex_string() {
        let val: HexStr<u8> = crate::from_str("+CCID: 0x8d").unwrap();
        assert_eq!(*val, 0x8d);
        let val: HexStr<u16> = crate::from_str("+CCID: 0x0B00").unwrap();
        assert_eq!(*val, 0x0B00);
        let val: HexStr<u32> = crate::from_str("+CCID: D3AdB3ef").unwrap();
        assert_eq!(*val, 0xd3adb3ef);
        let val: HexStr<u64> = crate::from_str("+CCID: 0xFeedfACECAfeBE3F").unwrap();
        assert_eq!(*val, 0xFeedfACECAfeBE3F);
        let val: HexStr<u128> =
            crate::from_str("+CCID: 0x1234567890abcdef1234567890abcdef").unwrap();
        assert_eq!(*val, 0x1234567890abcdef1234567890abcdef);
    }
}
