use heapless::{ArrayLength, Vec};

pub trait SliceExt {
    fn trim(&self, whitespaces: &[u8]) -> &Self;
    fn trim_start(&self, whitespaces: &[u8]) -> &Self;
}

impl SliceExt for [u8] {
    fn trim(&self, whitespaces: &[u8]) -> &[u8] {
        let is_not_whitespace = |c| !whitespaces.contains(c);

        if let Some(first) = self.iter().position(is_not_whitespace) {
            if let Some(last) = self.iter().rposition(is_not_whitespace) {
                &self[first..=last]
            } else {
                unreachable!();
            }
        } else {
            &[]
        }
    }

    fn trim_start(&self, whitespaces: &[u8]) -> &[u8] {
        let is_not_whitespace = |c| !whitespaces.contains(c);
        self.iter()
            .position(is_not_whitespace)
            .map_or(&[], |first| &self[first..])
    }
}

/// Helper function to take a subsection from `buf`.
///
/// It searches for `needle`, either from the beginning of buf, or the end,
/// depending on `reverse`. If the search finds a match, it continues forward as
/// long as the next characters matches `line_term_char` or `format_char`. It
/// then returns a substring, trimming it for whitespaces if `trim_response` is
/// true, and leaves the remainder in `buf`.
///
/// Example:
/// ```
/// use atat::helpers::get_line;
/// use heapless::{consts, Vec};
///
/// let mut buf: Vec<u8, consts::U128> =
///     Vec::from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\nAT+GMR\r\r\n").unwrap();
/// let response: Option<Vec<u8, consts::U64>> =
///     get_line(&mut buf, b"OK", b'\r', b'\n', false, false, false);
/// assert_eq!(
///     response,
///     Some(Vec::from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n").unwrap())
/// );
/// assert_eq!(
///     buf,
///     Vec::<u8, consts::U128>::from_slice(b"AT+GMR\r\r\n").unwrap()
/// );
/// ```
pub fn get_line<L: ArrayLength<u8>, I: ArrayLength<u8>>(
    buf: &mut Vec<u8, I>,
    needle: &[u8],
    line_term_char: u8,
    format_char: u8,
    trim_response: bool,
    reverse: bool,
    swap: bool,
) -> Option<Vec<u8, L>> {
    if buf.len() == 0 {
        return None;
    }

    let ind = if reverse {
        buf.windows(needle.len())
            .rposition(|window| window == needle)
    } else {
        buf.windows(needle.len())
            .position(|window| window == needle)
    };

    #[cfg(test)]
    println!("{:?}", ind);

    match ind {
        Some(index) => {
            let white_space = buf
                .iter()
                .skip(index + needle.len())
                .skip_while(|c| ![format_char, line_term_char, b'>', b'@'].contains(c))
                .position(|c| ![format_char, line_term_char].contains(c))
                .unwrap_or(buf.len() - index - needle.len());

            let (left, right) = match buf.split_at(index + needle.len() + white_space) {
                (left, right) if !swap => (left, right),
                (left, right) if swap => (right, left),
                _ => return None,
            };

            let return_buf = if trim_response {
                left.trim(&[b'\t', b' ', format_char, line_term_char])
            } else {
                left
            }
            .iter()
            // Truncate the response, rather than panic in case of buffer overflow!
            .take(L::to_usize())
            .cloned()
            .collect();

            *buf = right.iter().cloned().collect();
            Some(return_buf)
        }
        None => None,
    }
}

#[cfg(feature = "log")]
#[macro_export]
macro_rules! atat_log {
    ($level:ident, $($arg:expr),*) => {
        log::$level!($($arg),*);
    }
}
#[cfg(all(feature = "defmt", not(feature = "log")))]
#[macro_export]
macro_rules! atat_log {
    ($level:ident, $($arg:expr),*) => {
        defmt::$level!($($arg),*);
    }
}
#[cfg(not(any(feature = "defmt", feature = "log")))]
#[macro_export]
macro_rules! atat_log {
    ($level:ident, $($arg:expr),*) => {
        {
            $( let _ = $arg; )*
            ()
        }

    }
}
#[cfg(all(feature = "defmt", feature = "log"))]
compile_error!("You must enable at most one of the following features: defmt-*, log");

/// Wrapper for a byte-slice that formats it as a string if possible and as
/// bytes otherwise.
pub struct LossyStr<'a>(pub &'a [u8]);

impl<'a> core::fmt::Debug for LossyStr<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match core::str::from_utf8(self.0) {
            Ok(s) => write!(f, "{:?}", s),
            Err(_) => write!(f, "{:?}", self.0),
        }
    }
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for LossyStr<'a> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{=[u8]:a}", self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn trim() {
        assert_eq!(
            b"  hello  whatup  ".trim(&[b' ', b'\t', b'\r', b'\n']),
            b"hello  whatup"
        );
        assert_eq!(
            b"  hello  whatup  ".trim_start(&[b' ', b'\t', b'\r', b'\n']),
            b"hello  whatup  "
        );
        assert_eq!(
            b"  \r\n \thello  whatup  ".trim_start(&[b' ', b'\t', b'\r', b'\n']),
            b"hello  whatup  "
        );
        assert_eq!(
            b"  \r\n \thello  whatup  \n \t".trim(&[b' ', b'\t', b'\r', b'\n']),
            b"hello  whatup"
        );
    }
}
