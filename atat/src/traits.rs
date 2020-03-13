use crate::error::{NBResult, Result};
use crate::Mode;
use heapless::{ArrayLength, String};

pub trait ATATErr {}

/// This trait needs to be implemented for every response type.
pub trait ATATResp {}

pub trait ATATUrc {
    type Resp;

    fn parse(resp: &str) -> Result<Self::Resp>;
}

/// This trait needs to be implemented for every command type.
pub trait ATATCmd {
    /// The max length of the command.
    ///
    /// Example: For the command "AT+RST" you would specify
    ///
    /// ```
    /// type CommandLen = heapless::consts::U6;
    /// ```
    type CommandLen: ArrayLength<u8>;

    /// The type of the response. Must implement the `ATATResp` trait.
    type Response: ATATResp;

    /// Return the command as a heapless `String`.
    fn as_str(&self) -> String<Self::CommandLen>;

    /// Parse the string response into a `Self::Response` instance.
    fn parse(&self, resp: &str) -> Result<Self::Response>;

    /// Whether or not this command can be aborted.
    fn can_abort(&self) -> bool {
        false
    }

    /// The max timeout in milliseconds.
    fn max_timeout_ms(&self) -> u32 {
        1000
    }
}

pub trait ATATInterface {
    fn send<A: ATATCmd>(&mut self, cmd: &A) -> NBResult<A::Response>;

    fn check_urc<URC: ATATUrc>(&mut self) -> Option<URC::Resp>;

    fn check_response<A: ATATCmd>(&mut self, cmd: &A) -> NBResult<A::Response>;

    fn get_mode(&self) -> Mode;
}
