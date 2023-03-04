//! Serialize a Rust data structure into AT Command strings

use core::fmt::{self, Write};

use serde::ser;

use heapless::{String, Vec};

mod enum_;
mod hex_str;
mod struct_;

use self::enum_::{SerializeStructVariant, SerializeTupleVariant};
use self::struct_::SerializeStruct;

/// Serialization result
pub type Result<T> = ::core::result::Result<T, Error>;

/// Options used by the serializer, to customize the resulting string
pub struct SerializeOptions<'a> {
    /// Wether or not to include `=` as a seperator between the at command, and
    /// the parameters (serialized struct fields)
    ///
    /// **default**: true
    pub value_sep: bool,
    /// The prefix, added before the command.
    ///
    /// **default**: "AT"
    pub cmd_prefix: &'a str,
    /// The termination characters to add after the last serialized parameter.
    ///
    /// **default**: "\r\n"
    pub termination: &'a str,
}

impl<'a> Default for SerializeOptions<'a> {
    fn default() -> Self {
        SerializeOptions {
            value_sep: true,
            cmd_prefix: "AT",
            termination: "\r\n",
        }
    }
}

/// This type represents all possible errors that can occur when serializing AT
/// Command strings
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Buffer is full
    BufferFull,
}

impl From<()> for Error {
    fn from(_: ()) -> Self {
        Self::BufferFull
    }
}

impl From<u8> for Error {
    fn from(_: u8) -> Self {
        Self::BufferFull
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Buffer is full")
    }
}

pub(crate) struct Serializer<'a, const B: usize> {
    buf: Vec<u8, B>,
    cmd: &'a str,
    options: SerializeOptions<'a>,
}

impl<'a, const B: usize> Serializer<'a, B> {
    fn new(cmd: &'a str, options: SerializeOptions<'a>) -> Self {
        Serializer {
            buf: Vec::new(),
            cmd,
            options,
        }
    }
}

// NOTE(serialize_*signed) This is basically the numtoa implementation minus the lookup tables,
// which take 200+ bytes of ROM / Flash
macro_rules! serialize_unsigned {
    ($self:ident, $N:expr, $v:expr) => {{
        let mut buf: [u8; $N] = unsafe { super::uninitialized() };

        let mut v = $v;
        let mut i = $N - 1;
        loop {
            buf[i] = (v % 10) as u8 + b'0';
            v /= 10;

            if v == 0 {
                break;
            }
            i -= 1;
        }

        $self.buf.extend_from_slice(&buf[i..])?;
        Ok(())
    }};
}

macro_rules! serialize_signed {
    ($self:ident, $N:expr, $v:expr, $ixx:ident, $uxx:ident) => {{
        let v = $v;
        let (signed, mut v) = if v == $ixx::min_value() {
            (true, $ixx::max_value() as $uxx + 1)
        } else if v < 0 {
            (true, -v as $uxx)
        } else {
            (false, v as $uxx)
        };

        let mut buf: [u8; $N] = unsafe { super::uninitialized() };
        let mut i = $N - 1;
        loop {
            buf[i] = (v % 10) as u8 + b'0';
            v /= 10;

            i -= 1;

            if v == 0 {
                break;
            }
        }

        if signed {
            buf[i] = b'-';
        } else {
            i += 1;
        }
        $self.buf.extend_from_slice(&buf[i..])?;
        Ok(())
    }};
}

macro_rules! serialize_fmt {
    ($self:ident, $N:expr, $fmt:expr, $v:expr) => {{
        let mut s: String<$N> = String::new();
        write!(&mut s, $fmt, $v).unwrap();
        $self.buf.extend_from_slice(s.as_bytes())?;
        Ok(())
    }};
}

impl<'a, 'b, const B: usize> ser::Serializer for &'a mut Serializer<'b, B> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Unreachable;
    type SerializeTuple = Unreachable;
    type SerializeTupleStruct = Unreachable;
    type SerializeTupleVariant = SerializeTupleVariant<'a, 'b, B>;
    type SerializeMap = Unreachable;
    type SerializeStruct = SerializeStruct<'a, 'b, B>;
    type SerializeStructVariant = SerializeStructVariant<'a, 'b, B>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        if v {
            self.buf.extend_from_slice(b"true")?;
        } else {
            self.buf.extend_from_slice(b"false")?;
        }

        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        // "-128"
        serialize_signed!(self, 4, v, i8, u8)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        // "-32768"
        serialize_signed!(self, 6, v, i16, u16)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        // "-2147483648"
        serialize_signed!(self, 11, v, i32, u32)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        // "-9223372036854775808"
        serialize_signed!(self, 20, v, i64, u64)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        // "255"
        serialize_unsigned!(self, 3, v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        // "65535"
        serialize_unsigned!(self, 5, v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        // "4294967295"
        serialize_unsigned!(self, 10, v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        // "18446744073709551615"
        serialize_unsigned!(self, 20, v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        serialize_fmt!(self, 16, "{:e}", v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        serialize_fmt!(self, 32, "{:e}", v)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        let mut encoding_tmp = [0_u8; 4];
        let encoded = v.encode_utf8(&mut encoding_tmp as &mut [u8]);
        self.buf.extend_from_slice(encoded.as_bytes())?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        let mut encoding_tmp = [0_u8; 4];
        for c in v.chars() {
            let encoded = c.encode_utf8(&mut encoding_tmp as &mut [u8]);
            self.buf.extend_from_slice(encoded.as_bytes())?;
        }
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        self.buf.extend_from_slice(v)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.buf.truncate(self.buf.len() - 1);
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ser::Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        unreachable!()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.buf
            .extend_from_slice(self.options.cmd_prefix.as_bytes())?;
        self.buf.extend_from_slice(self.cmd.as_bytes())?;
        self.buf
            .extend_from_slice(self.options.termination.as_bytes())?;
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_u32(variant_index)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ser::Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: ser::Serialize + ?Sized,
    {
        self.serialize_u32(variant_index)?;
        self.buf.push(b',')?;
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        unreachable!()
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        unreachable!()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        unreachable!()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.serialize_u32(variant_index)?;
        self.buf.push(b',')?;
        Ok(SerializeTupleVariant::new(self))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        unreachable!()
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.buf
            .extend_from_slice(self.options.cmd_prefix.as_bytes())?;
        self.buf.extend_from_slice(self.cmd.as_bytes())?;
        Ok(SerializeStruct::new(self))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.serialize_u32(variant_index)?;
        self.buf.push(b',')?;
        Ok(SerializeStructVariant::new(self))
    }

    fn collect_str<T: ?Sized>(self, _value: &T) -> Result<Self::Ok> {
        unreachable!()
    }
}

/// Serializes the given data structure as a string
pub fn to_string<T, const B: usize>(
    value: &T,
    cmd: &str,
    options: SerializeOptions<'_>,
) -> Result<String<B>>
where
    T: ser::Serialize + ?Sized,
{
    let mut ser = Serializer::<B>::new(cmd, options);
    value.serialize(&mut ser)?;
    Ok(String::from(unsafe {
        core::str::from_utf8_unchecked(&ser.buf)
    }))
}

/// Serializes the given data structure as a byte vector
pub fn to_vec<T, const B: usize>(
    value: &T,
    cmd: &str,
    options: SerializeOptions<'_>,
) -> Result<Vec<u8, B>>
where
    T: ser::Serialize + ?Sized,
{
    let mut ser = Serializer::new(cmd, options);
    value.serialize(&mut ser)?;
    Ok(ser.buf)
}

impl ser::Error for Error {
    fn custom<T>(_msg: T) -> Self {
        unreachable!()
    }
}

impl ser::StdError for Error {}

#[allow(clippy::empty_enum)]
pub(crate) enum Unreachable {}

impl ser::SerializeTupleStruct for Unreachable {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<()> {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok> {
        unreachable!()
    }
}

impl ser::SerializeMap for Unreachable {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, _key: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        unreachable!()
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok> {
        unreachable!()
    }
}

impl ser::SerializeSeq for Unreachable {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, _value: &T) -> Result<()> {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok> {
        unreachable!()
    }
}

impl ser::SerializeTuple for Unreachable {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, _value: &T) -> Result<()> {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok> {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HexStr;
    use heapless::String;
    use serde_bytes::Bytes;
    use serde_derive::{Deserialize, Serialize};

    #[derive(Clone, PartialEq, Serialize, Deserialize)]
    pub enum PacketSwitchedParam {
        /// • 0: Protocol type; the allowed values of <param_val> parameter are
        // #[at_enum(0)]
        ProtocolType(bool),
        /// • 1: APN - <param_val> defines the APN text string, e.g. "apn.provider.com"; the
        /// maximum length is 99. The factory-programmed value is an empty string.
        APN(String<128>),
        /// • 2: username - <param_val> is the user name text string for the authentication
        /// phase. The factory-programmed value is an empty string.
        Username(String<128>),
        /// • 3: password - <param_val> is the password text string for the authentication phase.
        /// Note: the AT+UPSD read command with param_tag = 3 is not allowed and the read
        /// all command does not display it
        Password(String<128>),

        QoSDelay3G(u32),
        CurrentProfileMap(u8),
        AppEui(HexStr<u32>),
    }

    #[derive(Clone, PartialEq, Serialize, Deserialize)]
    pub enum PinStatusCode {
        /// • READY: MT is not pending for any password
        #[serde(rename = "READY")]
        Ready,
        /// • SIM PIN: MT is waiting SIM PIN to be given
        #[serde(rename = "SIM PIN")]
        SimPin,
        /// • SIM PUK: MT is waiting SIM PUK to be given
        /// • SIM PIN2: MT is waiting SIM PIN2 to be given
        /// • SIM PUK2: MT is waiting SIM PUK2 to be given
        /// • PH-NET PIN: MT is waiting network personalization password to be given
        /// • PH-NETSUB PIN: MT is waiting network subset personalization password to be
        /// given
        /// • PH-SP PIN: MT is waiting service provider personalization password to be given
        /// • PH-CORP PIN: MT is waiting corporate personalization password to be given
        /// • PH-SIM PIN: MT is waiting phone to SIM/UICC card password to be given
        #[serde(rename = "PH-SIM PIN")]
        PhSimPin,
    }

    #[derive(Clone, PartialEq, Serialize, Deserialize)]
    struct Handle(pub usize);

    #[test]
    fn tuple_struct() {
        let s: String<32> = to_string(
            &PacketSwitchedParam::QoSDelay3G(15),
            "",
            SerializeOptions::default(),
        )
        .unwrap();

        assert_eq!(s, String::<32>::from("4,15"));
    }

    #[test]
    fn newtype_struct() {
        let s: String<32> = to_string(&Handle(15), "", SerializeOptions::default()).unwrap();

        assert_eq!(s, String::<32>::from("15"));
    }

    #[test]
    fn byte_serialize() {
        #[derive(Clone, PartialEq, Serialize)]
        pub struct WithBytes<'a> {
            s: &'a Bytes,
        }
        let slice = b"Some bytes";
        let b = WithBytes {
            s: Bytes::new(&slice[..]),
        };
        let s: String<32> = to_string(&b, "+CMD", SerializeOptions::default()).unwrap();
        assert_eq!(s, String::<32>::from("AT+CMD=Some bytes\r\n"));
    }

    #[test]
    fn hex_str_serialize() {
        #[derive(Clone, PartialEq, Serialize)]
        pub struct WithHexStr {
            val_0x_caps: HexStr<u32>,
            val_no_0x_caps: HexStr<u32>,
            val_0x_small_case: HexStr<u32>,
            val_no_0x_small_case: HexStr<u32>,
            val_0x_caps_delimiter: HexStr<u32>,
            val_no_0x_caps_delimiter: HexStr<u32>,
            val_0x_small_case_delimiter: HexStr<u32>,
            val_no_0x_small_case_delimiter: HexStr<u64>,
        }

        let params = WithHexStr {
            val_0x_caps: HexStr {
                val: 0xFF00,
                hex_in_caps: true,
                add_0x_with_encoding: true,
                delimiter: ' ',
                delimiter_after_nibble_count: 0
            },
            val_no_0x_caps: HexStr {
                val: 0x55AA,
                hex_in_caps: true,
                add_0x_with_encoding: false,
                delimiter: ' ',
                delimiter_after_nibble_count: 0
            },
            val_0x_small_case: HexStr {
                val: 0x00FF,
                hex_in_caps: false,
                add_0x_with_encoding: true,
                delimiter: ' ',
                delimiter_after_nibble_count: 0
            },
            val_no_0x_small_case: HexStr {
                val: 0xAA55,
                hex_in_caps: false,
                add_0x_with_encoding: false,
                delimiter: ' ',
                delimiter_after_nibble_count: 0
            },
            val_0x_caps_delimiter: HexStr {
                val: 0xFF00,
                hex_in_caps: true,
                add_0x_with_encoding: true,
                delimiter: ':',
                delimiter_after_nibble_count: 1
            },
            val_no_0x_caps_delimiter: HexStr {
                val: 0x55AA,
                hex_in_caps: true,
                add_0x_with_encoding: false,
                delimiter: '-',
                delimiter_after_nibble_count: 2
            },
            val_0x_small_case_delimiter: HexStr {
                val: 0x00FF,
                hex_in_caps: false,
                add_0x_with_encoding: true,
                delimiter: ':',
                delimiter_after_nibble_count: 1
            },
            val_no_0x_small_case_delimiter: HexStr {
                val: 0xAA5500FF,
                hex_in_caps: false,
                add_0x_with_encoding: false,
                delimiter: '-',
                delimiter_after_nibble_count: 2
            },
        };

        let s: String<200> = to_string(&params, "+CMD", SerializeOptions::default()).unwrap();
        assert_eq!(s, String::<100>::from("AT+CMD=0xFF00,55AA,0xff,aa55,0xF:F:0:0,55-AA,0xf:f,aa-55-00-ff\r\n"));
    }
}
