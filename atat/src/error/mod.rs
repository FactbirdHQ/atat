mod cme_error;
mod cms_error;
mod connection_error;

pub use cme_error::CmeError;
pub use cms_error::CmsError;
pub use connection_error::ConnectionError;
use core::fmt;

/// Errors returned used internally within the crate
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl<'a> fmt::Display for InternalError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::Read => write!(f, "Serial read error"),
            InternalError::Write => write!(f, "Serial write error"),
            InternalError::Timeout => write!(f, "Timed out while waiting for a response"),
            InternalError::InvalidResponse => write!(f, "Invalid response from module"),
            InternalError::Aborted => write!(f, "Command was aborted"),
            InternalError::Parse => write!(f, "Failed to parse received response"),
            InternalError::Error => write!(f, "Generic error response"),
            InternalError::CmeError(e) => write!(f, "GSM Equipment related error: {:?}", e),
            InternalError::CmsError(e) => write!(f, "GSM Network related error: {:?}", e),
            InternalError::ConnectionError(e) => write!(f, "Connection Error: {:?}", e),
            InternalError::Custom(e) => write!(f, "Custom error match: {:?}", e),
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Failed to parse received response
    Parse,
    /// Generic error response without any error message
    Error,
    /// GSM Equipment related error
    CmeError(CmeError),
    /// GSM Network related error
    CmsError(CmsError),
    /// Connection Error
    ConnectionError(ConnectionError),
    /// Error response containing any error message
    Custom,
    #[cfg(feature = "custom-error-messages")]
    CustomMessage(heapless::Vec<u8, 64>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read => write!(f, "Serial read error"),
            Self::Write => write!(f, "Serial write error"),
            Self::Timeout => write!(f, "Timed out while waiting for a response"),
            Self::InvalidResponse => write!(f, "Invalid response from module"),
            Self::Aborted => write!(f, "Command was aborted"),
            Self::Parse => write!(f, "Failed to parse received response"),
            Self::Error => write!(f, "Generic error response"),
            Self::CmeError(e) => write!(f, "GSM Equipment related error: {:?}", e),
            Self::CmsError(e) => write!(f, "GSM Network related error: {:?}", e),
            Self::ConnectionError(e) => write!(f, "Connection Error: {:?}", e),
            Self::Custom => write!(f, "Custom error response"),
            #[cfg(feature = "custom-error-messages")]
            Self::CustomMessage(msg) => {
                write!(f, "Error response containing any error message {:?}", msg)
            }
        }
    }
}

impl core::error::Error for Error {}

impl embedded_io::Error for Error {
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            Self::Read => embedded_io::ErrorKind::Other,
            Self::Write => embedded_io::ErrorKind::Other,
            _ => embedded_io::ErrorKind::Other,
        }
    }
}

impl<'a> From<InternalError<'a>> for Error {
    fn from(ie: InternalError) -> Self {
        match ie {
            InternalError::Read => Self::Read,
            InternalError::Write => Self::Write,
            InternalError::Timeout => Self::Timeout,
            InternalError::InvalidResponse => Self::InvalidResponse,
            InternalError::Aborted => Self::Aborted,
            InternalError::Parse => Self::Parse,
            InternalError::Error => Self::Error,
            InternalError::CmeError(e) => Self::CmeError(e),
            InternalError::CmsError(e) => Self::CmsError(e),
            InternalError::ConnectionError(e) => Self::ConnectionError(e),
            #[cfg(feature = "custom-error-messages")]
            InternalError::Custom(e) => Self::CustomMessage(
                heapless::Vec::from_slice(&e[..core::cmp::min(e.len(), 64)]).unwrap_or_default(),
            ),
            #[cfg(not(feature = "custom-error-messages"))]
            InternalError::Custom(_) => Self::Custom,
        }
    }
}
