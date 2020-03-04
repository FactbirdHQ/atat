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

    #[cfg(feature = "error-message")]
    InvalidResponseWithMessage(heapless::String<heapless::consts::U256>),

    ResponseError,

    Aborted,

    Overflow,

    ParseString,
}

pub type Result<T> = core::result::Result<T, Error>;
pub type NBResult<T> = nb::Result<T, Error>;
