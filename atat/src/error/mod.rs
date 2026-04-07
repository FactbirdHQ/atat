mod cme_error;
mod cms_error;
mod connection_error;

pub use cme_error::CmeError;
pub use cms_error::CmsError;
pub use connection_error::ConnectionError;
use thiserror::Error;

/// Errors returned used internally within the crate
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum InternalError<'a> {
    /// Serial read error
    #[error("Serial read error")]
    Read,
    /// Serial write error
    #[error("Serial write error")]
    Write,
    /// Timed out while waiting for a response
    #[error("Timed out while waiting for a response")]
    Timeout,
    /// Invalid response from module
    #[error("Invalid response from module")]
    InvalidResponse,
    /// Command was aborted
    #[error("Command was aborted")]
    Aborted,
    /// Failed to parse received response
    #[error("Failed to parse received response")]
    Parse,
    /// Error response containing any error message
    #[error("Generic error response")]
    Error,
    /// GSM Equipment related error
    #[error("GSM Equipment related error")]
    CmeError(CmeError),
    /// GSM Network related error
    #[error("GSM Network related error")]
    CmsError(CmsError),
    /// Connection Error
    #[error("Connection Error")]
    ConnectionError(ConnectionError),
    /// Custom error match
    #[error("Custom error match: {0:?}")]
    Custom(&'a [u8]),
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
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Serial read error
    #[error("Serial read error")]
    Read,
    /// Serial write error
    #[error("Serial write error")]
    Write,
    /// Timed out while waiting for a response
    #[error("Timed out while waiting for a response")]
    Timeout,
    /// Invalid response from module
    #[error("Invalid response from module")]
    InvalidResponse,
    /// Command was aborted
    #[error("Command was aborted")]
    Aborted,
    /// Failed to parse received response
    #[error("Failed to parse received response")]
    Parse,
    /// Generic error response without any error message
    #[error("Generic error response")]
    Error,
    /// GSM Equipment related error
    #[error("GSM Equipment related error")]
    CmeError(CmeError),
    /// GSM Network related error
    #[error("GSM Network related error")]
    CmsError(CmsError),
    /// Connection Error
    #[error("Connection Error")]
    ConnectionError(ConnectionError),
    /// Error response containing any error message
    #[error("Custom error response")]
    Custom,
    #[cfg(feature = "custom-error-messages")]
    #[error("Error response containing any error message {0:?}")]
    CustomMessage(heapless::Vec<u8, 64>),
}

impl embedded_io::Error for Error {
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            Self::Timeout => embedded_io::ErrorKind::TimedOut,
            Self::InvalidResponse => embedded_io::ErrorKind::InvalidData,
            Self::Aborted => embedded_io::ErrorKind::ConnectionAborted,
            Self::Parse => embedded_io::ErrorKind::InvalidData,
            Self::ConnectionError(e) => match e {
                ConnectionError::Unknown => embedded_io::ErrorKind::NotConnected,
                ConnectionError::NoCarrier => embedded_io::ErrorKind::ConnectionReset,
                ConnectionError::NoDialtone => embedded_io::ErrorKind::NotConnected,
                ConnectionError::Busy => embedded_io::ErrorKind::Other,
                ConnectionError::NoAnswer => embedded_io::ErrorKind::TimedOut,
            },
            _ => embedded_io::ErrorKind::Other,
            #[cfg(feature = "custom-error-messages")]
            Self::CustomMessage(_) => embedded_io::ErrorKind::Other,
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

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_io::{Error as _, ErrorKind};

    #[test]
    fn test_error_kind_mapping() {
        assert_eq!(Error::Read.kind(), ErrorKind::Other);
        assert_eq!(Error::Write.kind(), ErrorKind::Other);
        assert_eq!(Error::Timeout.kind(), ErrorKind::TimedOut);
        assert_eq!(Error::InvalidResponse.kind(), ErrorKind::InvalidData);
        assert_eq!(Error::Aborted.kind(), ErrorKind::ConnectionAborted);
        assert_eq!(Error::Parse.kind(), ErrorKind::InvalidData);
        assert_eq!(Error::Error.kind(), ErrorKind::Other);
        assert_eq!(
            Error::ConnectionError(ConnectionError::Unknown).kind(),
            ErrorKind::NotConnected
        );
        assert_eq!(
            Error::ConnectionError(ConnectionError::NoCarrier).kind(),
            ErrorKind::ConnectionReset
        );
        assert_eq!(
            Error::ConnectionError(ConnectionError::NoDialtone).kind(),
            ErrorKind::NotConnected
        );
        assert_eq!(
            Error::ConnectionError(ConnectionError::Busy).kind(),
            ErrorKind::Other
        );
        assert_eq!(
            Error::ConnectionError(ConnectionError::NoAnswer).kind(),
            ErrorKind::TimedOut
        );
        assert_eq!(Error::Custom.kind(), ErrorKind::Other);
    }
}
