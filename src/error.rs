#[derive(Debug, PartialEq)]
pub enum Error {
    /// Serial read error
    Read,

    /// Serial write error
    Write,

    Busy,

    Timeout,

    /// Invalid response from module
    InvalidResponse,

    Overflow,

    ParseString,
}
