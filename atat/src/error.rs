/// Errors returned by, or used within the crate
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    any(
        feature = "defmt-default",
        feature = "defmt-trace",
        feature = "defmt-debug",
        feature = "defmt-info",
        feature = "defmt-warn",
        feature = "defmt-error"
    ),
    derive(defmt::Format)
)]
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
