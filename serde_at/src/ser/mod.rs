//! Serialize a Rust data structure into AT Command strings

use core::fmt;

use serde::ser;

mod enum_;
#[cfg(feature = "heapless")]
mod hex_str;
mod struct_;

use self::enum_::{SerializeStructVariant, SerializeTupleVariant};
use self::struct_::SerializeStruct;

/// Serialization result
pub type Result<T> = ::core::result::Result<T, Error>;

/// Options used by the serializer, to customize the resulting string
pub struct SerializeOptions<'a> {
    /// Whether or not to include `=` as a seperator between the at command, and
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
    /// Whether to add quotes before a string when serializing a string
    ///
    /// **default**: true
    pub quote_escape_strings: bool,
}

impl<'a> Default for SerializeOptions<'a> {
    fn default() -> Self {
        SerializeOptions {
            value_sep: true,
            cmd_prefix: "AT",
            termination: "\r\n",
            quote_escape_strings: true,
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Buffer is full")
    }
}

pub(crate) struct Serializer<'a> {
    buf: &'a mut [u8],
    written: usize,
    nested_struct: bool,
    cmd: &'a str,
    options: SerializeOptions<'a>,
}

impl<'a> Serializer<'a> {
    fn new(buf: &'a mut [u8], cmd: &'a str, options: SerializeOptions<'a>) -> Self {
        Serializer {
            buf,
            written: 0,
            nested_struct: false,
            cmd,
            options,
        }
    }

    fn push(&mut self, c: u8) -> Result<()> {
        if self.written < self.buf.len() {
            self.buf[self.written] = c;
            self.written += 1;
            Ok(())
        } else {
            Err(Error::BufferFull)
        }
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> Result<()> {
        if self.written + other.len() <= self.buf.len() {
            self.buf[self.written..self.written + other.len()].copy_from_slice(other);
            self.written += other.len();
            Ok(())
        } else {
            Err(Error::BufferFull)
        }
    }

    fn write_buf(&mut self) -> &mut [u8] {
        &mut self.buf[self.written..]
    }

    fn commit(&mut self, amount: usize) -> Result<()> {
        if self.written + amount <= self.buf.len() {
            self.written += amount;
            Ok(())
        } else {
            Err(Error::BufferFull)
        }
    }
}

// NOTE(serialize_*signed) This is basically the numtoa implementation minus the lookup tables,
// which take 200+ bytes of ROM / Flash
macro_rules! serialize_unsigned {
    ($self:ident, $N:expr, $v:expr) => {{
        let mut buf = super::uninit_array::<u8, $N>();

        let mut v = $v;
        let mut i = $N - 1;
        loop {
            buf[i].write((v % 10) as u8 + b'0');
            v /= 10;

            if v == 0 {
                break;
            }
            i -= 1;
        }

        // SAFETY: The buffer was initialized from `i` to the end.
        let out = unsafe { super::slice_assume_init_ref(&buf[i..]) };

        $self.extend_from_slice(out)?;
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

        let mut buf = super::uninit_array::<u8, $N>();
        let mut i = $N - 1;
        loop {
            buf[i].write((v % 10) as u8 + b'0');
            v /= 10;

            i -= 1;

            if v == 0 {
                break;
            }
        }

        if signed {
            buf[i].write(b'-');
        } else {
            i += 1;
        }

        // SAFETY: The buffer was initialized from `i` to the end.
        let out = unsafe { super::slice_assume_init_ref(&buf[i..]) };

        $self.extend_from_slice(out)?;
        Ok(())
    }};
}

struct FmtWrapper<'a> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> FmtWrapper<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        FmtWrapper {
            buf: buf,
            offset: 0,
        }
    }
}

impl<'a> fmt::Write for FmtWrapper<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();

        // Skip over already-copied data
        let remainder = &mut self.buf[self.offset..];
        // Check if there is space remaining (return error instead of panicking)
        if remainder.len() < bytes.len() {
            return Err(core::fmt::Error);
        }
        // Make the two slices the same length
        let remainder = &mut remainder[..bytes.len()];
        // Copy
        remainder.copy_from_slice(bytes);

        // Update offset to avoid overwriting
        self.offset += bytes.len();

        Ok(())
    }
}

macro_rules! serialize_fmt {
    ($self:ident, $fmt:expr, $v:expr) => {{
        use fmt::Write;
        let mut wrapper = FmtWrapper::new($self.write_buf());
        write!(wrapper, $fmt, $v).unwrap();
        let written = wrapper.offset;
        $self.commit(written)
    }};
}

impl<'a, 'b> ser::Serializer for &'a mut Serializer<'b> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Unreachable;
    type SerializeTuple = Unreachable;
    type SerializeTupleStruct = Unreachable;
    type SerializeTupleVariant = SerializeTupleVariant<'a, 'b>;
    type SerializeMap = Unreachable;
    type SerializeStruct = SerializeStruct<'a, 'b>;
    type SerializeStructVariant = SerializeStructVariant<'a, 'b>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        if v {
            self.extend_from_slice(b"true")?;
        } else {
            self.extend_from_slice(b"false")?;
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
        serialize_fmt!(self, "{}", v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        serialize_fmt!(self, "{}", v)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        let mut encoding_tmp = [0_u8; 4];
        let encoded = v.encode_utf8(&mut encoding_tmp as &mut [u8]);
        self.extend_from_slice(encoded.as_bytes())?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        if self.options.quote_escape_strings {
            self.push(b'"')?;
        }
        let mut encoding_tmp = [0_u8; 4];
        for c in v.chars() {
            let encoded = c.encode_utf8(&mut encoding_tmp as &mut [u8]);
            self.extend_from_slice(encoded.as_bytes())?;
        }
        if self.options.quote_escape_strings {
            self.push(b'"')?;
        }
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        self.extend_from_slice(v)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.written -= 1;
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
        self.extend_from_slice(self.options.cmd_prefix.as_bytes())?;
        self.extend_from_slice(self.cmd.as_bytes())?;
        self.extend_from_slice(self.options.termination.as_bytes())?;
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
        self.push(b',')?;
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
        self.push(b',')?;
        Ok(SerializeTupleVariant::new(self))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        unreachable!()
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        let ser_struct = if !self.nested_struct {
            // all calls to serialize_struct after this one will be nested structs
            self.nested_struct = true;
            self.extend_from_slice(self.options.cmd_prefix.as_bytes())?;
            self.extend_from_slice(self.cmd.as_bytes())?;
            SerializeStruct::new(self, false)
        } else {
            SerializeStruct::new(self, true)
        };
        Ok(ser_struct)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.serialize_u32(variant_index)?;
        self.push(b',')?;
        Ok(SerializeStructVariant::new(self))
    }

    fn collect_str<T: ?Sized>(self, _value: &T) -> Result<Self::Ok> {
        unreachable!()
    }
}

#[cfg(feature = "heapless")]
/// Serializes the given data structure as a string
pub fn to_string<T, const N: usize>(
    value: &T,
    cmd: &str,
    options: SerializeOptions<'_>,
) -> Result<heapless::String<N>>
where
    T: ser::Serialize + ?Sized,
{
    let vec: heapless::Vec<u8, N> = to_vec(value, cmd, options)?;
    Ok(unsafe { heapless::String::from_utf8_unchecked(vec) })
}

#[cfg(feature = "heapless")]
/// Serializes the given data structure as a byte vector
pub fn to_vec<T, const N: usize>(
    value: &T,
    cmd: &str,
    options: SerializeOptions<'_>,
) -> Result<heapless::Vec<u8, N>>
where
    T: ser::Serialize + ?Sized,
{
    let mut buf = heapless::Vec::new();
    buf.resize_default(N).map_err(|_| Error::BufferFull)?;
    let len = to_slice(value, cmd, &mut buf, options)?;
    buf.truncate(len);
    Ok(buf)
}

/// Serializes the given data structure to a buffer
pub fn to_slice<T>(
    value: &T,
    cmd: &str,
    buf: &mut [u8],
    options: SerializeOptions<'_>,
) -> Result<usize>
where
    T: ser::Serialize + ?Sized,
{
    let mut ser = Serializer::new(buf, cmd, options);
    value.serialize(&mut ser)?;
    Ok(ser.written)
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

#[cfg(all(test, feature = "heapless"))]
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

        assert_eq!(s, String::<32>::try_from("4,15").unwrap());
    }

    #[test]
    fn newtype_struct() {
        let s: String<32> = to_string(&Handle(15), "", SerializeOptions::default()).unwrap();

        assert_eq!(s, String::<32>::try_from("15").unwrap());
    }

    #[test]
    fn struct_with_none_option() {
        #[derive(Clone, PartialEq, Serialize)]
        pub struct WithOption<'a> {
            s: Option<&'a str>,
        }

        let value = WithOption { s: None };

        let s: String<32> = to_string(&value, "+CMD", SerializeOptions::default()).unwrap();

        assert_eq!(s, String::<32>::try_from("AT+CMD\r\n").unwrap());
    }

    #[test]
    fn struct_with_some_option() {
        #[derive(Clone, PartialEq, Serialize)]
        pub struct WithOption<'a> {
            s: Option<&'a str>,
        }

        let value = WithOption { s: Some("value") };

        let s: String<32> = to_string(&value, "+CMD", SerializeOptions::default()).unwrap();

        assert_eq!(s, String::<32>::try_from("AT+CMD=\"value\"\r\n").unwrap());
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
        assert_eq!(s, String::<32>::try_from("AT+CMD=Some bytes\r\n").unwrap());
    }

    #[test]
    fn nested_struct() {
        #[derive(Clone, PartialEq, Serialize)]
        pub struct Inner<'a> {
            s: &'a str,
        }

        #[derive(Clone, PartialEq, Serialize)]
        pub struct Outer<'a> {
            inner: Inner<'a>,
        }

        let value = Outer {
            inner: Inner { s: "value" },
        };

        let s: String<32> = to_string(&value, "+CMD", SerializeOptions::default()).unwrap();

        assert_eq!(s, String::<32>::try_from("AT+CMD=\"value\"\r\n").unwrap());
    }

    #[test]
    fn fmt_float() {
        #[derive(Clone, PartialEq, Serialize)]
        pub struct Floats {
            f32: f32,
            f64: f64,
        }

        let value = Floats {
            f32: 1.23,
            f64: 4.56,
        };

        let s: String<32> = to_string(&value, "+CMD", SerializeOptions::default()).unwrap();

        assert_eq!(s, String::<32>::try_from("AT+CMD=1.23,4.56\r\n").unwrap());
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
                delimiter_after_nibble_count: 0,
                skip_last_0_values: false,
            },
            val_no_0x_caps: HexStr {
                val: 0x55AA,
                hex_in_caps: true,
                add_0x_with_encoding: false,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: false,
            },
            val_0x_small_case: HexStr {
                val: 0x00FF,
                hex_in_caps: false,
                add_0x_with_encoding: true,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: false,
            },
            val_no_0x_small_case: HexStr {
                val: 0xAA55,
                hex_in_caps: false,
                add_0x_with_encoding: false,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: false,
            },
            val_0x_caps_delimiter: HexStr {
                val: 0xFF00,
                hex_in_caps: true,
                add_0x_with_encoding: true,
                delimiter: ':',
                delimiter_after_nibble_count: 1,
                skip_last_0_values: false,
            },
            val_no_0x_caps_delimiter: HexStr {
                val: 0x55AA,
                hex_in_caps: true,
                add_0x_with_encoding: false,
                delimiter: '-',
                delimiter_after_nibble_count: 2,
                skip_last_0_values: false,
            },
            val_0x_small_case_delimiter: HexStr {
                val: 0x00FF,
                hex_in_caps: false,
                add_0x_with_encoding: true,
                delimiter: ':',
                delimiter_after_nibble_count: 1,
                skip_last_0_values: false,
            },
            val_no_0x_small_case_delimiter: HexStr {
                val: 0xAA5500FF,
                hex_in_caps: false,
                add_0x_with_encoding: false,
                delimiter: '-',
                delimiter_after_nibble_count: 2,
                skip_last_0_values: false,
            },
        };
        let options = SerializeOptions {
            quote_escape_strings: false,
            ..Default::default()
        };
        let s: String<600> = to_string(&params, "+CMD", options).unwrap();
        assert_eq!(
            s,
            String::<600>::try_from(
                "AT+CMD=0x0000FF00,000055AA,0x000000ff,0000aa55,0x0:0:0:0:F:F:0:0,00-00-55-AA,0x0:0:0:0:0:0:f:f,00-00-00-00-aa-55-00-ff\r\n"
            ).unwrap()
        );

        let params = WithHexStr {
            val_0x_caps: HexStr {
                val: 0xFF00,
                hex_in_caps: true,
                add_0x_with_encoding: true,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: true,
            },
            val_no_0x_caps: HexStr {
                val: 0x55AA,
                hex_in_caps: true,
                add_0x_with_encoding: false,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: true,
            },
            val_0x_small_case: HexStr {
                val: 0x00FF,
                hex_in_caps: false,
                add_0x_with_encoding: true,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: true,
            },
            val_no_0x_small_case: HexStr {
                val: 0xAA55,
                hex_in_caps: false,
                add_0x_with_encoding: false,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: true,
            },
            val_0x_caps_delimiter: HexStr {
                val: 0xFF00,
                hex_in_caps: true,
                add_0x_with_encoding: true,
                delimiter: ':',
                delimiter_after_nibble_count: 1,
                skip_last_0_values: true,
            },
            val_no_0x_caps_delimiter: HexStr {
                val: 0x55AA,
                hex_in_caps: true,
                add_0x_with_encoding: false,
                delimiter: '-',
                delimiter_after_nibble_count: 2,
                skip_last_0_values: true,
            },
            val_0x_small_case_delimiter: HexStr {
                val: 0x00FF,
                hex_in_caps: false,
                add_0x_with_encoding: true,
                delimiter: ':',
                delimiter_after_nibble_count: 1,
                skip_last_0_values: true,
            },
            val_no_0x_small_case_delimiter: HexStr {
                val: 0xAA5500FF,
                hex_in_caps: false,
                add_0x_with_encoding: false,
                delimiter: '-',
                delimiter_after_nibble_count: 2,
                skip_last_0_values: true,
            },
        };
        let options = SerializeOptions {
            quote_escape_strings: false,
            ..Default::default()
        };
        let s: String<600> = to_string(&params, "+CMD", options).unwrap();
        assert_eq!(
            s,
            String::<600>::try_from(
                "AT+CMD=0xFF00,55AA,0xff,aa55,0xF:F:0:0,55-AA,0xf:f,aa-55-00-ff\r\n"
            )
            .unwrap()
        );
    }

    #[cfg(feature = "hex_str_arrays")]
    #[test]
    fn hex_str_serialize_byte_array() {
        #[derive(Clone, PartialEq, Serialize)]
        pub struct WithHexStr {
            val_no_0x_small_caps_drop_last_0s: HexStr<[u8; 16]>,
            val_0x_caps_array: HexStr<[u8; 8]>,
            val_0x_small_caps_array: HexStr<[u8; 8]>,
            val_no_0x_caps_array: HexStr<[u8; 8]>,
            val_no_0x_small_caps_array: HexStr<[u8; 8]>,
            val_0x_caps_array_delimiter: HexStr<[u8; 8]>,
            val_0x_small_caps_array_delimiter: HexStr<[u8; 8]>,
            val_no_0x_caps_array_delimiter: HexStr<[u8; 8]>,
            val_no_0x_small_caps_array_delimiter: HexStr<[u8; 8]>,
        }

        let options = SerializeOptions {
            quote_escape_strings: false,
            ..Default::default()
        };
        let params = WithHexStr {
            val_no_0x_small_caps_drop_last_0s: HexStr {
                val: [
                    0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
                hex_in_caps: false,
                add_0x_with_encoding: false,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: true,
            },
            val_0x_caps_array: HexStr {
                val: [0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55],
                hex_in_caps: true,
                add_0x_with_encoding: true,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: false,
            },
            val_0x_small_caps_array: HexStr {
                val: [0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55],
                hex_in_caps: false,
                add_0x_with_encoding: true,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: false,
            },
            val_no_0x_caps_array: HexStr {
                val: [0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55],
                hex_in_caps: true,
                add_0x_with_encoding: false,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: false,
            },
            val_no_0x_small_caps_array: HexStr {
                val: [0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55],
                hex_in_caps: false,
                add_0x_with_encoding: false,
                delimiter: ' ',
                delimiter_after_nibble_count: 0,
                skip_last_0_values: false,
            },
            val_0x_caps_array_delimiter: HexStr {
                val: [0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55],
                hex_in_caps: true,
                add_0x_with_encoding: true,
                delimiter: '-',
                delimiter_after_nibble_count: 2,
                skip_last_0_values: false,
            },
            val_0x_small_caps_array_delimiter: HexStr {
                val: [0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55],
                hex_in_caps: false,
                add_0x_with_encoding: true,
                delimiter: ':',
                delimiter_after_nibble_count: 1,
                skip_last_0_values: false,
            },
            val_no_0x_caps_array_delimiter: HexStr {
                val: [0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55],
                hex_in_caps: true,
                add_0x_with_encoding: false,
                delimiter: '_',
                delimiter_after_nibble_count: 2,
                skip_last_0_values: false,
            },
            val_no_0x_small_caps_array_delimiter: HexStr {
                val: [0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55],
                hex_in_caps: false,
                add_0x_with_encoding: false,
                delimiter: '#',
                delimiter_after_nibble_count: 1,
                skip_last_0_values: false,
            },
        };
        let s: String<600> = to_string(&params, "+CMD", options).unwrap();
        assert_eq!(
            s,
            String::<600>::try_from(
                "AT+CMD=ff00aa55ff00aa55,0xFF00AA55FF00AA55,0xff00aa55ff00aa55,FF00AA55FF00AA55,ff00aa55ff00aa55,0xFF-00-AA-55-FF-00-AA-55,0xf:f:0:0:a:a:5:5:f:f:0:0:a:a:5:5,FF_00_AA_55_FF_00_AA_55,f#f#0#0#a#a#5#5#f#f#0#0#a#a#5#5\r\n"
            ).unwrap()
        );
    }
}
