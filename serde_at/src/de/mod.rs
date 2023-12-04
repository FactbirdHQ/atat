//! Deserialize AT Command strings to a Rust data structure

use core::str::FromStr;
use core::{fmt, str};

use serde::de::{self, Visitor};

use self::enum_::VariantAccess;
use self::map::MapAccess;
use self::seq::SeqAccess;

mod enum_;
pub mod length_delimited;
mod map;
mod seq;

/// Hex string helper module
pub mod hex_str;

/// Deserialization result
pub type Result<T> = core::result::Result<T, Error>;

/// This type represents all possible errors that can occur when deserializing AT Command strings
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// EOF while parsing an object.
    EofWhileParsingObject,

    /// EOF while parsing a string.
    EofWhileParsingString,

    /// EOF while parsing an AT Command string.
    EofWhileParsingNumber,

    /// EOF while parsing an AT Command string.
    EofWhileParsingValue,

    /// Expected to parse either a `true`, `false`, or a `null`.
    ExpectedSomeIdent,

    /// Expected this character to start an AT Command string.
    ExpectedSomeValue,

    /// Invalid number.
    InvalidNumber,

    /// Invalid type
    InvalidType,

    /// Invalid unicode code point.
    InvalidUnicodeCodePoint,

    /// AT Command string has non-whitespace trailing characters after the value.
    TrailingCharacters,

    /// AT Command string has a comma after the last value in an array or map.
    TrailingComma,

    /// Error with a custom message that we had to discard.
    CustomError,

    /// Error with a custom message that was preserved.
    #[cfg(feature = "custom-error-messages")]
    CustomErrorWithMessage(heapless::String<128>),
}

pub(crate) struct Deserializer<'b> {
    slice: &'b [u8],
    index: usize,
    struct_size_hint: Option<usize>,
    is_trailing_parsing: bool,
}

impl<'a> Deserializer<'a> {
    const fn new(slice: &'a [u8]) -> Deserializer<'_> {
        Deserializer {
            slice,
            index: 0,
            struct_size_hint: None,
            is_trailing_parsing: false,
        }
    }

    fn eat_char(&mut self) {
        self.index += 1;
    }

    fn end(&mut self) -> Result<()> {
        match self.parse_whitespace() {
            Some(_) => Err(Error::TrailingCharacters),
            None => Ok(()),
        }
    }

    fn next_char(&mut self) -> Option<&u8> {
        if let Some(ch) = self.slice.get(self.index) {
            self.index += 1;
            return Some(ch);
        }
        None
    }

    fn set_is_trailing_parsing(&mut self) {
        self.is_trailing_parsing = true;
    }

    fn struct_size_hint(&self) -> Option<usize> {
        self.struct_size_hint
    }

    fn parse_ident(&mut self, ident: &[u8]) -> Result<()> {
        for c in ident {
            if Some(c) != self.next_char() {
                return Err(Error::ExpectedSomeIdent);
            }
        }

        Ok(())
    }

    fn parse_str(&mut self) -> Result<&'a str> {
        let start = self.index;
        if self.is_trailing_parsing {
            self.index = self.slice.len();
            return str::from_utf8(&self.slice[start..])
                .map_err(|_| Error::InvalidUnicodeCodePoint);
        } else {
            loop {
                match self.peek() {
                    Some(b'"') => {
                        // Counts the number of backslashes in front of the current index.
                        //
                        // "some string with \\\" included."
                        //                  ^^^^^
                        //                  |||||
                        //       loop run:  4321|
                        //                      |
                        //                   `index`
                        //
                        // Since we only get in this code branch if we found a " starting the string and `index` is greater
                        // than the start position, we know the loop will end no later than this point.
                        let leading_backslashes = |index: usize| -> usize {
                            let mut count = 0;
                            loop {
                                if self.slice[index - count - 1] == b'\\' {
                                    count += 1;
                                } else {
                                    return count;
                                }
                            }
                        };

                        let is_escaped = leading_backslashes(self.index) % 2 == 1;
                        if is_escaped {
                            self.eat_char(); // just continue
                        } else {
                            let end = self.index;
                            self.eat_char();
                            return str::from_utf8(&self.slice[start..end])
                                .map_err(|_| Error::InvalidUnicodeCodePoint);
                        }
                    }
                    Some(_) => self.eat_char(),
                    None => {
                        return Err(Error::EofWhileParsingString);
                    }
                }
            }
        }
    }

    fn parse_bytes(&mut self) -> Result<&'a [u8]> {
        let start = self.index;
        loop {
            if self.is_trailing_parsing {
                self.index = self.slice.len();
                return Ok(&self.slice[start..]);
            } else {
                if let Some(c) = self.peek() {
                    if (c as char).is_alphanumeric() || (c as char).is_whitespace() {
                        self.eat_char();
                    } else {
                        return Err(Error::EofWhileParsingString);
                    }
                } else {
                    return Ok(&self.slice[start..self.index]);
                }
            }
        }
    }

    fn parse_at(&mut self) -> Result<Option<()>> {
        // If we find a '+', check if it is an AT command identifier, ending in ':'
        if self.parse_whitespace() == Some(b'+') {
            let index = self.index;
            loop {
                match self.peek() {
                    Some(b':') => {
                        self.eat_char();
                        self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;
                        return Ok(Some(()));
                    }
                    Some(_) => {
                        self.eat_char();
                    }
                    None => {
                        // Doesn't seem to be an AT command identifier. Reset index and continue
                        self.index = index;
                        break;
                    }
                }
            }
        }
        Ok(None)
    }

    /// Consumes all the whitespace characters and returns a peek into the next character
    fn parse_whitespace(&mut self) -> Option<u8> {
        loop {
            match self.peek() {
                Some(b' ' | b'\n' | b'\t' | b'\r') => {
                    self.eat_char();
                }
                other => {
                    return other;
                }
            }
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.slice.get(self.index).copied()
    }
}

// NOTE(deserialize_*signed) we avoid parsing into u64 and then casting to a smaller integer, which
// is what upstream does, to avoid pulling in 64-bit compiler intrinsics, which waste a few KBs of
// Flash, when targeting non 64-bit architectures
macro_rules! deserialize_unsigned {
    ($self:ident, $visitor:ident, $uxx:ident, $visit_uxx:ident) => {{
        let peek = $self
            .parse_whitespace()
            .ok_or(Error::EofWhileParsingValue)?;

        match peek {
            b'-' => Err(Error::InvalidNumber),
            b'0' => {
                $self.eat_char();
                $visitor.$visit_uxx(0)
            }
            b'1'..=b'9' => {
                $self.eat_char();

                let mut number = (peek - b'0') as $uxx;
                loop {
                    match $self.peek() {
                        Some(c @ b'0'..=b'9') => {
                            $self.eat_char();
                            number = number
                                .checked_mul(10)
                                .ok_or(Error::InvalidNumber)?
                                .checked_add((c - b'0') as $uxx)
                                .ok_or(Error::InvalidNumber)?;
                        }
                        _ => return $visitor.$visit_uxx(number),
                    }
                }
            }
            _ => Err(Error::InvalidType),
        }
    }};
}

macro_rules! deserialize_signed {
    ($self:ident, $visitor:ident, $ixx:ident, $visit_ixx:ident) => {{
        let signed = match $self
            .parse_whitespace()
            .ok_or(Error::EofWhileParsingValue)?
        {
            b'-' => {
                $self.eat_char();
                true
            }
            _ => false,
        };

        match $self.peek().ok_or(Error::EofWhileParsingValue)? {
            b'0' => {
                $self.eat_char();
                $visitor.$visit_ixx(0)
            }
            c @ b'1'..=b'9' => {
                $self.eat_char();

                let mut number = (c - b'0') as $ixx * if signed { -1 } else { 1 };
                loop {
                    match $self.peek() {
                        Some(c @ b'0'..=b'9') => {
                            $self.eat_char();
                            number = number
                                .checked_mul(10)
                                .ok_or(Error::InvalidNumber)?
                                .checked_add((c - b'0') as $ixx * if signed { -1 } else { 1 })
                                .ok_or(Error::InvalidNumber)?;
                        }
                        _ => return $visitor.$visit_ixx(number),
                    }
                }
            }
            _ => return Err(Error::InvalidType),
        }
    }};
}

macro_rules! deserialize_fromstr {
    ($self:ident, $visitor:ident, $typ:ident, $visit_fn:ident, $pattern:expr) => {{
        let start = $self.index;
        loop {
            match $self.peek() {
                Some(c) => {
                    if $pattern.iter().find(|&&d| d == c).is_some() {
                        $self.eat_char();
                    } else {
                        let s = unsafe {
                            // already checked that it contains only ascii
                            str::from_utf8_unchecked(&$self.slice[start..$self.index])
                        };
                        let v = $typ::from_str(s).or(Err(Error::InvalidNumber))?;
                        return $visitor.$visit_fn(v);
                    }
                }
                None => return Err(Error::EofWhileParsingNumber),
            }
        }
    }};
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    /// Unsupported. Can’t parse a value without knowing its expected type.
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let peek = self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;
        match peek {
            b't' => {
                self.eat_char();
                self.parse_ident(b"rue")?;
                visitor.visit_bool(true)
            }
            b'f' => {
                self.eat_char();
                self.parse_ident(b"alse")?;
                visitor.visit_bool(false)
            }
            _ => Err(Error::InvalidType),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_signed!(self, visitor, i8, visit_i8)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_signed!(self, visitor, i16, visit_i16)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_signed!(self, visitor, i32, visit_i32)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_signed!(self, visitor, i64, visit_i64)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_signed!(self, visitor, i128, visit_i128)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned!(self, visitor, u8, visit_u8)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned!(self, visitor, u16, visit_u16)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned!(self, visitor, u32, visit_u32)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned!(self, visitor, u64, visit_u64)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned!(self, visitor, u128, visit_u128)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;
        deserialize_fromstr!(self, visitor, f32, visit_f32, b"0123456789+-.eE")
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;
        deserialize_fromstr!(self, visitor, f64, visit_f64, b"0123456789+-.eE")
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let peek = self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;
        self.eat_char();
        visitor.visit_char(peek as char)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let peek = self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;

        match peek {
            b'"' => {
                self.eat_char();
                visitor.visit_borrowed_str(self.parse_str()?)
            }
            _ => {
                if (peek as char).is_alphabetic() {
                    visitor.visit_bytes(self.parse_bytes()?)
                } else {
                    Err(Error::InvalidType)
                }
            }
        }
    }

    /// Unsupported. String is not available in no-std.
    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_at()?;
        let idx = self.slice[self.index..]
            .iter()
            .position(|b| *b == b',')
            .unwrap_or(self.slice.len() - self.index);

        visitor
            .visit_bytes(&self.slice[self.index..self.index + idx])
            .map(|r| {
                self.index += idx;
                r
            })
    }

    /// Unsupported
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.parse_whitespace() {
            Some(b'+' | b',') | None => visitor.visit_none(),
            Some(_) => visitor.visit_some(self),
        }
    }

    /// Unsupported. Use a more specific deserialize_* method
    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    /// Unsupported. Use a more specific deserialize_* method
    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_at()?;
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SeqAccess::new(self))
    }

    /// deserialize_tuple is (mis)used for parsing LengthDelimited types.
    /// They can only be used as the last param as we cannot yet communicate the length
    /// back to from the visitor to slice the slice.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor
            .visit_bytes(self.slice[self.index..].as_ref())
            .map(|v| {
                self.index = self.slice.len(); // Since we know it is the last param.
                v
            })
    }

    /// Unsupported
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;
        visitor.visit_map(MapAccess::new(self))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_at()?;

        // Misuse EofWhileParsingObject here to indicate finished object in vec
        // cases. Don't start a new sequence if this is not the first, and we
        // have passed the last character in the buffer
        //
        // TODO: is there a better way of doing this?!
        if self.index == self.slice.len() && self.index > 0 {
            return Err(Error::EofWhileParsingObject);
        }
        self.struct_size_hint = Some(fields.len());
        let result = self.deserialize_seq(visitor);
        self.struct_size_hint = None;

        result
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;
        visitor.visit_enum(VariantAccess::new(self))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.parse_whitespace().ok_or(Error::EofWhileParsingValue)? {
            b'"' => self.deserialize_str(visitor),
            b',' | b'}' | b']' => Err(Error::ExpectedSomeValue),
            _ => loop {
                match self.peek() {
                    // The visitor is expected to be UnknownAny’s visitor, which
                    // implements visit_unit to return its unit Ok result.
                    Some(b',' | b'}' | b']') => break visitor.visit_unit(),
                    Some(_) => self.eat_char(),
                    None => break Err(Error::EofWhileParsingString),
                }
            },
        }
    }
}

impl de::Error for Error {
    #[cfg_attr(not(feature = "custom-error-messages"), allow(unused_variables))]
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        #[cfg(not(feature = "custom-error-messages"))]
        {
            Self::CustomError
        }
        #[cfg(feature = "custom-error-messages")]
        {
            use core::fmt::Write;

            let mut string = heapless::String::new();
            write!(string, "{:.64}", msg).unwrap();
            Self::CustomErrorWithMessage(string)
        }
    }
}

impl de::StdError for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::EofWhileParsingObject => "EOF while parsing an object.",
                Self::EofWhileParsingString => "EOF while parsing a string.",
                Self::EofWhileParsingValue => "EOF while parsing an AT Command string.",
                Self::ExpectedSomeIdent => {
                    "Expected to parse either a `true`, `false`, or a \
                     `null`."
                }
                Self::ExpectedSomeValue => "Expected this character to start an AT Command string.",
                Self::InvalidNumber => "Invalid number.",
                Self::InvalidType => "Invalid type",
                Self::InvalidUnicodeCodePoint => "Invalid unicode code point.",
                Self::TrailingCharacters => {
                    "AT Command string has non-whitespace trailing characters after \
                     the \
                     value."
                }
                Self::CustomError =>
                    "AT Command string does not match deserializer\u{2019}s expected format.",
                #[cfg(feature = "custom-error-messages")]
                Self::CustomErrorWithMessage(msg) => msg.as_str(),
                _ => "Invalid AT Command string",
            }
        )
    }
}

fn trim_ascii_whitespace(x: &[u8]) -> &[u8] {
    x.iter().position(|x| !x.is_ascii_whitespace()).map_or_else(
        || &x[0..0],
        |from| {
            let to = x.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
            &x[from..=to]
        },
    )
}

/// Deserializes an instance of type `T` from bytes of AT Response text
pub fn from_slice<'a, T>(v: &'a [u8]) -> Result<T>
where
    T: de::Deserialize<'a>,
{
    let mut de = Deserializer::new(trim_ascii_whitespace(v));
    let value = de::Deserialize::deserialize(&mut de)?;
    de.end()?;
    Ok(value)
}

/// Deserializes an instance of type T from a string of AT Response text
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: de::Deserialize<'a>,
{
    from_slice(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::length_delimited::LengthDelimited;
    use heapless::String;
    use heapless_bytes::Bytes;
    use serde_derive::Deserialize;

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct CFG {
        p1: u8,
        p2: i16,
        p3: bool,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct CFGOption {
        p1: u8,
        p2: i16,
        p3: Option<bool>,
    }

    #[derive(Clone, Debug, Deserialize, PartialEq)]
    pub struct CCID {
        pub ccid: u128,
    }

    #[derive(Clone, Debug, PartialEq, Deserialize)]
    struct Handle(pub usize);

    #[derive(Clone, Debug, PartialEq, Deserialize)]
    struct CharHandle(pub char);

    #[test]
    fn simple_struct() {
        assert_eq!(
            crate::from_str("+CFG: 2,56,false"),
            Ok(CFG {
                p1: 2,
                p2: 56,
                p3: false
            })
        );
    }

    #[test]
    fn simple_struct_optionals() {
        assert_eq!(
            crate::from_str("+CFG: 2,56"),
            Ok(CFGOption {
                p1: 2,
                p2: 56,
                p3: None
            })
        );

        assert_eq!(
            crate::from_str("+CFG: 2,56, true"),
            Ok(CFGOption {
                p1: 2,
                p2: 56,
                p3: Some(true)
            })
        );
        assert_eq!(
            crate::from_str("+CFG: 2,56,false"),
            Ok(CFGOption {
                p1: 2,
                p2: 56,
                p3: Some(false)
            })
        );
    }
    #[test]
    fn simple_string() {
        #[derive(Clone, Debug, Deserialize, PartialEq)]
        pub struct StringTest {
            pub string: String<32>,
        }

        assert_eq!(
            crate::from_str("+CCID: \"89883030000005421166\""),
            Ok(StringTest {
                string: String::try_from("89883030000005421166").unwrap()
            })
        );
    }

    #[test]
    fn cgmi_string() {
        #[derive(Clone, Debug, Deserialize, PartialEq)]
        pub struct CGMI {
            pub id: Bytes<32>,
        }

        let expectation = CGMI {
            id: Bytes::from_slice(b"u-blox").unwrap(),
        };

        assert_eq!(core::str::from_utf8(&expectation.id), Ok("u-blox"));
        assert_eq!(crate::from_slice(b"u-blox"), Ok(expectation));
    }

    #[test]
    fn u128_test() {
        assert_eq!(
            crate::from_str("+CCID: 89883030000005421166"),
            Ok(CCID {
                ccid: 89883030000005421166
            })
        );
    }

    #[test]
    fn char_test() {
        assert_eq!(crate::from_str("+CCID: B"), Ok(CharHandle('B')));
    }

    #[test]
    fn newtype_struct() {
        assert_eq!(crate::from_str("+CCID: 15"), Ok(Handle(15)));
    }

    #[test]
    fn char_vec_struct() {
        let res: Bytes<4> = crate::from_str("+CCID: IMP_").unwrap();
        assert_eq!(res, Bytes::<4>::from_slice(b"IMP_").unwrap());

        assert_eq!(&res, b"IMP_");
    }

    #[test]
    fn trailing_cmgr_parsing() {
        #[derive(Clone, Debug, Deserialize, PartialEq)]
        pub struct Message {
            state: String<256>,
            sender: String<256>,
            size: Option<usize>,
            date: String<256>,
            message: String<256>,
        }

        assert_eq!(
            crate::from_str(
                "+CMGR: \"REC UNREAD\",\"+48788899722\",12,\"23/11/21,13:31:39+04\"\r\nINFO,WWW\"\"a"
            ),
            Ok(Message {
                state: String::try_from("REC UNREAD").unwrap(),
                sender: String::try_from("+48788899722").unwrap(),
                size: Some(12),
                date: String::try_from("23/11/21,13:31:39+04").unwrap(),
                message: String::try_from("INFO,WWW\"\"a").unwrap(),
            })
        );
    }

    #[test]
    fn length_delimited() {
        #[derive(Clone, Debug, Deserialize)]
        pub struct PayloadResponse {
            pub ctx: u8, // Some other params
            pub id: i8,  // Some other params
            pub payload: LengthDelimited<32>,
        }

        let res: PayloadResponse = crate::from_slice(b"1,-1,9,\"ABCD,1234\"").unwrap();
        assert_eq!(res.ctx, 1);
        assert_eq!(res.id, -1);
        assert_eq!(res.payload.len, 9);
        assert_eq!(
            res.payload.bytes,
            Bytes::<32>::from_slice(b"ABCD,1234").unwrap()
        );
    }

    #[test]
    fn length_delimited_json() {
        #[derive(Clone, Debug, Deserialize)]
        pub struct PayloadResponse {
            pub ctx: u8, // Some other params
            pub id: i8,  // Some other params
            pub payload: LengthDelimited<32>,
        }
        // This tests correct handling of commas in the payload.
        let res: PayloadResponse =
            crate::from_slice(b"1,-2,28,\"{\"cmd\": \"blink\", \"pin\": \"2\"}\"").unwrap();
        assert_eq!(res.ctx, 1);
        assert_eq!(res.id, -2);
        assert_eq!(res.payload.len, 28);
        assert_eq!(
            res.payload.bytes,
            Bytes::<32>::from_slice(b"{\"cmd\": \"blink\", \"pin\": \"2\"}").unwrap()
        );
    }
}
