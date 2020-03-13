/// Errors returned by, or used within the crate
#[derive(Clone, Debug, PartialEq)]
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
    ParseString,
}
