use heapless::{consts, String, Vec};

/// Errors returned by, or used within the crate
#[derive(Clone, Debug, PartialEq)]
pub enum IngressError {
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
    Error(Vec<u8, consts::U64>),
}

/// Errors returned by, or used within the crate
#[derive(Clone, Debug, PartialEq)]
pub enum Error<E> {
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
    Error(E),
}

impl<E> From<&IngressError> for Error<E>
where
    E: core::str::FromStr,
{
    fn from(ie: &IngressError) -> Self {
        match ie {
            &IngressError::Read => Self::Read,
            &IngressError::Write => Self::Write,
            &IngressError::Timeout => Self::Timeout,
            &IngressError::InvalidResponse => Self::InvalidResponse,
            &IngressError::Aborted => Self::Aborted,
            &IngressError::Overflow => Self::Overflow,
            &IngressError::Parse => Self::Parse,
            &IngressError::Error(ref e) => {
                if let Ok(s) = String::from_utf8(e.clone()) {
                    if let Ok(e) = core::str::FromStr::from_str(s.as_str()) {
                        return Self::Error(e);
                    }
                }
                Self::Parse
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, defmt::Format)]
pub struct GenericError;

impl core::str::FromStr for GenericError {
    type Err = core::convert::Infallible;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Ok(GenericError)
    }
}
