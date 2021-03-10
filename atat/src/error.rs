use heapless::{consts, String, Vec};

/// Errors returned used internally within the crate
#[derive(Clone, Debug, PartialEq)]
pub enum InternalError {
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
    Error(Vec<u8, consts::U85>),
}

/// Errors returned by the crate
#[derive(Clone, Debug, PartialEq, defmt::Format)]
pub enum Error<E = GenericError>
where
    E: defmt::Format,
{
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

impl<E> From<&InternalError> for Error<E>
where
    E: core::str::FromStr + defmt::Format,
{
    fn from(ie: &InternalError) -> Self {
        match ie {
            &InternalError::Read => Self::Read,
            &InternalError::Write => Self::Write,
            &InternalError::Timeout => Self::Timeout,
            &InternalError::InvalidResponse => Self::InvalidResponse,
            &InternalError::Aborted => Self::Aborted,
            &InternalError::Overflow => Self::Overflow,
            &InternalError::Parse => Self::Parse,
            &InternalError::Error(ref e) => {
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
