//! Parsing of length delimited byte strings.
//!
use core::fmt;

use heapless_bytes::Bytes;
use serde::{de, Deserialize, Deserializer};

/// Structure for parsing a length delimited bytes payload.
///
/// This supports both quoted and non-quoted payloads.
/// For "quoted" payloads the length is assumed to be excluding the surrounding double quotes.
///
/// For example:
///
/// For both this response: `+QMTRECV: 1,0,"topic",4,"ABCD"`
/// and this response: `+QTRECV: 1,0,"topic",4,ABCD`
///
/// We can parse the last two parameters as a 'LengthDelimited' object which yields:
/// `'4,"ABCD"' => LengthDelimited { len: 4, bytes: [65, 66, 67, 68] }`
///
#[derive(Clone, Debug)]
pub struct LengthDelimited<const N: usize, const S: usize = 1> {
    /// The number of bytes in the payload. This is actually
    /// redundant since the `bytes` field also knows its own length.
    pub len: usize,
    /// The payload bytes
    pub bytes: Bytes<N>,
}

impl<'de, const N: usize, const S: usize> Deserialize<'de> for LengthDelimited<N, S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Ideally we use deserializer.deserialize_bytes but since it clips the payload
        // at the first comma we cannot use it.
        // Instead we use deserialize_tuple as it wasn't used yet.
        deserializer.deserialize_tuple(2, LengthDelimitedVisitor::<N, S>) // The '2' is dummy.
    }
}

struct LengthDelimitedVisitor<const N: usize, const L: usize>;

impl<'de, const N: usize, const S: usize> de::Visitor<'de> for LengthDelimitedVisitor<N, S> {
    type Value = LengthDelimited<N, S>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("length delimited bytes, e.g.: \"4,ABCD\"")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.iter()
            .position(|&c| !c.is_ascii_digit())
            .ok_or_else(|| de::Error::custom("expected a comma"))
            .and_then(|pos| {
                let len = parse_len(&v[0..pos])
                    .map_err(|_| de::Error::custom("expected an unsigned int"))?;
                // +S to skip the separator after the length.
                let mut start = pos + S;
                let mut end = start + len;
                // Check if payload is surrounded by double quotes not included in len.
                let slice_len = v.len();
                if slice_len >= (end + 2) && (v[start] == b'"' && v[end + 1] == b'"') {
                    start += 1; // Extra +1 to remove first quote (")
                    end += 1; // Move end by 1 to compensate for the quote.
                }
                Ok(LengthDelimited {
                    len,
                    bytes: Bytes::from_slice(&v[start..end])
                        .map_err(|_| de::Error::custom("incorrect slice size"))?,
                })
            })
    }
}

/// Parses a slice of bytes into an unsigned integer.
/// The slice must contain only ASCII _digits_ and must not contain additional bytes.
fn parse_len(v: &[u8]) -> Result<usize, ()> {
    let len_str: &str = core::str::from_utf8(v).map_err(|_| ())?;
    len_str.parse().map_err(|_| ())
}
