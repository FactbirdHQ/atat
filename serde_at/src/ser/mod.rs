//! Serialize a Rust data structure into AT Command strings

use core::fmt::{self, Write};

use serde::ser;

use heapless::{consts::*, String, Vec};

use self::struct_::SerializeStruct;

mod struct_;

/// Serialization result
pub type Result<T> = ::core::result::Result<T, Error>;

/// This type represents all possible errors that can occur when serializing AT Command strings
#[derive(Debug)]
pub enum Error {
    /// Buffer is full
    BufferFull,
    #[doc(hidden)]
    __Extensible,
}

impl From<()> for Error {
    fn from(_: ()) -> Error {
        Error::BufferFull
    }
}

impl From<u8> for Error {
    fn from(_: u8) -> Error {
        Error::BufferFull
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Buffer is full")
    }
}

pub(crate) struct Serializer<B, C>
where
    B: heapless::ArrayLength<u8>,
    C: heapless::ArrayLength<u8>,
{
    buf: Vec<u8, B>,
    cmd: String<C>,
    value_sep: bool,
}

impl<B, C> Serializer<B, C>
where
    B: heapless::ArrayLength<u8>,
    C: heapless::ArrayLength<u8>,
{
    fn new(cmd: String<C>, value_sep: bool) -> Self {
        Serializer {
            buf: Vec::new(),
            cmd,
            value_sep,
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
            } else {
                i -= 1;
            }
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
    ($self:ident, $uxx:ident, $fmt:expr, $v:expr) => {{
        let mut s: String<$uxx> = String::new();
        write!(&mut s, $fmt, $v).unwrap();
        $self.buf.extend_from_slice(s.as_bytes())?;
        Ok(())
    }};
}

impl<'a, B, C> ser::Serializer for &'a mut Serializer<B, C>
where
    B: heapless::ArrayLength<u8>,
    C: heapless::ArrayLength<u8>,
{
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Unreachable;
    type SerializeTuple = Unreachable;
    type SerializeTupleStruct = Unreachable;
    type SerializeTupleVariant = Unreachable;
    type SerializeMap = Unreachable;
    type SerializeStruct = SerializeStruct<'a, B, C>;
    type SerializeStructVariant = Unreachable;

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
        serialize_fmt!(self, U16, "{:e}", v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        serialize_fmt!(self, U32, "{:e}", v)
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok> {
        unreachable!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.buf.push(b'"')?;
        self.buf.extend_from_slice(v.as_bytes())?;
        self.buf.push(b'"')?;
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

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        unreachable!()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.buf.extend_from_slice(b"AT")?;
        self.buf.extend_from_slice(&self.cmd.as_bytes())?;
        self.buf.push(b'\r')?;
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

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: ser::Serialize,
    {
        self.serialize_u32(variant_index)?;
        self.buf.push(b',')?;
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        log::info!("Seq\r");
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
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        unreachable!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        unreachable!()
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.buf.extend_from_slice(b"AT")?;
        self.buf.extend_from_slice(&self.cmd.as_bytes())?;
        Ok(SerializeStruct::new(self))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        unreachable!()
    }

    fn collect_str<T: ?Sized>(self, _value: &T) -> Result<Self::Ok> {
        unreachable!()
    }
}

/// Serializes the given data structure as a string
pub fn to_string<B, C, T>(value: &T, cmd: String<C>, value_sep: bool) -> Result<String<B>>
where
    B: heapless::ArrayLength<u8>,
    C: heapless::ArrayLength<u8>,
    T: ser::Serialize + ?Sized,
{
    let mut ser = Serializer::new(cmd, value_sep);
    value.serialize(&mut ser)?;
    Ok(unsafe { String::from_utf8_unchecked(ser.buf) })
}

/// Serializes the given data structure as a byte vector
pub fn to_vec<B, C, T>(value: &T, cmd: String<C>, value_sep: bool) -> Result<Vec<u8, B>>
where
    B: heapless::ArrayLength<u8>,
    C: heapless::ArrayLength<u8>,
    T: ser::Serialize + ?Sized,
{
    let mut ser = Serializer::new(cmd, value_sep);
    value.serialize(&mut ser)?;
    Ok(ser.buf)
}

impl ser::Error for Error {
    fn custom<T>(_msg: T) -> Self {
        unreachable!()
    }
}

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

impl ser::SerializeTupleVariant for Unreachable {
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

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        unreachable!()
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok> {
        unreachable!()
    }
}

impl ser::SerializeStructVariant for Unreachable {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, _value: &T) -> Result<()>
    where
        T: ser::Serialize,
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
    use heapless::{consts, String};
    use serde_derive::{Deserialize, Serialize};

    #[derive(Clone, PartialEq, Serialize, Deserialize)]
    pub enum PacketSwitchedParam {
        /// • 0: Protocol type; the allowed values of <param_val> parameter are
        // #[at_enum(0)]
        ProtocolType(bool),
        /// • 1: APN - <param_val> defines the APN text string, e.g. "apn.provider.com"; the
        /// maximum length is 99. The factory-programmed value is an empty string.
        APN(String<consts::U128>),
        /// • 2: username - <param_val> is the user name text string for the authentication
        /// phase. The factory-programmed value is an empty string.
        Username(String<consts::U128>),
        /// • 3: password - <param_val> is the password text string for the authentication phase.
        /// Note: the AT+UPSD read command with param_tag = 3 is not allowed and the read
        /// all command does not display it
        Password(String<consts::U128>),

        QoSDelay3G(u32),
        CurrentProfileMap(u8),
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
        let s: String<consts::U32> = to_string(
            &PacketSwitchedParam::QoSDelay3G(15),
            String::<consts::U32>::from(""),
            true,
        )
        .unwrap();

        assert_eq!(s, String::<consts::U32>::from("4,15"));
    }

    #[test]
    fn newtype_struct() {
        let s: String<consts::U32> =
            to_string(&Handle(15), String::<consts::U32>::from(""), true).unwrap();

        assert_eq!(s, String::<consts::U32>::from("15"));
    }
}
