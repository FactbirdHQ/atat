mod cme_error;
mod cms_error;
mod connection_error;

pub use cme_error::CmeError;
pub use cms_error::CmsError;
pub use connection_error::ConnectionError;

/// Errors returned used internally within the crate
#[derive(Clone, Debug, PartialEq)]
pub enum InternalError<'a> {
    /// Serial read error
    Read,
    /// Serial write error
    Write,
    /// Timed out while waiting for a response
    Timeout,
    /// Invalid response from module
    InvalidResponse,
    /// Command was aborted
    Aborted,
    /// Buffer overflow
    Overflow,
    /// Failed to parse received response
    Parse,
    /// Error response containing any error message
    Error,
    /// GSM Equipment related error
    CmeError(CmeError),
    /// GSM Network related error
    CmsError(CmsError),
    /// Connection Error
    ConnectionError(ConnectionError),
    /// Custom error match
    Custom(&'a [u8]),
}

impl<'a> InternalError<'a> {
    pub fn as_byte(&self) -> u8 {
        match self {
            InternalError::Read => 0x00,
            InternalError::Write => 0x01,
            InternalError::Timeout => 0x02,
            InternalError::InvalidResponse => 0x03,
            InternalError::Aborted => 0x04,
            InternalError::Overflow => 0x05,
            InternalError::Parse => 0x06,
            InternalError::Error => 0x07,
            InternalError::CmeError(_) => 0x08,
            InternalError::CmsError(_) => 0x09,
            InternalError::ConnectionError(_) => 0x10,
            InternalError::Custom(_) => 0x11,
        }
    }

    pub fn from_bytes(b: &'a [u8]) -> Self {
        match &b[0] {
            0x00 => InternalError::Read,
            0x01 => InternalError::Write,
            0x02 => InternalError::Timeout,
            0x03 => InternalError::InvalidResponse,
            0x04 => InternalError::Aborted,
            0x05 => InternalError::Overflow,
            0x06 => InternalError::Parse,
            0x07 => InternalError::Error,
            0x08 => InternalError::ConnectionError(ConnectionError::Busy),
            0x09 if b.len() > 1 => InternalError::Custom(&b[1..]),
            0x09 => InternalError::Custom(&[]),
            _ => InternalError::Parse,
        }
    }
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for InternalError<'a> {
    fn format(&self, f: defmt::Formatter) {
        match self {
            InternalError::Read => defmt::write!(f, "InternalError::Read"),
            InternalError::Write => defmt::write!(f, "InternalError::Write"),
            InternalError::Timeout => defmt::write!(f, "InternalError::Timeout"),
            InternalError::InvalidResponse => defmt::write!(f, "InternalError::InvalidResponse"),
            InternalError::Aborted => defmt::write!(f, "InternalError::Aborted"),
            InternalError::Overflow => defmt::write!(f, "InternalError::Overflow"),
            InternalError::Parse => defmt::write!(f, "InternalError::Parse"),
            InternalError::Error => defmt::write!(f, "InternalError::Error"),
            InternalError::CmeError(e) => defmt::write!(f, "InternalError::CmeError({:?})", e),
            InternalError::CmsError(e) => defmt::write!(f, "InternalError::CmsError({:?})", e),
            InternalError::ConnectionError(e) => {
                defmt::write!(f, "InternalError::ConnectionError({:?})", e)
            }
            InternalError::Custom(e) => {
                defmt::write!(f, "InternalError::Custom({=[u8]:a})", &e)
            }
        }
    }
}

/// Errors returned by the crate
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Serial read error
    Read,
    /// Serial write error
    Write,
    /// Timed out while waiting for a response
    Timeout,
    /// Invalid response from module
    InvalidResponse,
    /// Command was aborted
    Aborted,
    /// Buffer overflow
    Overflow,
    /// Failed to parse received response
    Parse,
    /// Generic error response without any error message
    Error,
    /// Error response containing any error message
    Custom,
}

impl<'a> From<InternalError<'a>> for Error {
    fn from(ie: InternalError) -> Self {
        match ie {
            InternalError::Read => Self::Read,
            InternalError::Write => Self::Write,
            InternalError::Timeout => Self::Timeout,
            InternalError::InvalidResponse => Self::InvalidResponse,
            InternalError::Aborted => Self::Aborted,
            InternalError::Overflow => Self::Overflow,
            InternalError::Parse => Self::Parse,
            InternalError::Error => Self::Error,
            // FIXME:
            InternalError::CmeError(_) => Self::Error,
            InternalError::CmsError(_) => Self::Error,
            InternalError::ConnectionError(_) => Self::Error,
            InternalError::Custom(_) => Self::Error,
        }
    }
}
