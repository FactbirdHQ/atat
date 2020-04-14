use crate::error::Error;
use crate::Mode;
use heapless::{ArrayLength, Vec};

pub trait AtatErr {}

/// This trait needs to be implemented for every response type.
///
/// Example:
/// ```
/// use atat::prelude::*;
///
/// pub struct GreetingText {
///     pub text: heapless::String<heapless::consts::U64>
/// }
///
/// impl AtatResp for GreetingText {}
/// ```
pub trait AtatResp {}

pub trait AtatUrc {
    /// The type of the response. Usually the enum this trait is implemented on.
    type Response;

    /// Parse the string response into a `Self::Response` instance.
    fn parse(resp: &[u8]) -> Result<Self::Response, Error>;
}

/// This trait needs to be implemented for every command type.
///
/// It can also be derived by the [`atat_derive`] crate.
///
/// [`atat_derive`]: https://crates.io/crates/atat_derive
///
/// Example:
/// ```
/// use atat::prelude::*;
///
/// pub struct SetGreetingText<'a> {
///     pub text: &'a str
/// }
///
/// impl<'a> AtatCmd for SetGreetingText<'a> {
///     type CommandLen = heapless::consts::U64;
///     type Response = NoResponse;
///
///     fn as_str(&self) -> String<Self::CommandLen> {
///         let buf: String<Self::CommandLen> = String::new();
///         write!(buf, "AT+CSGT={}", self.text);
///         buf
///     }
///
///     fn parse(&self, resp: &str) -> Result<Self::Response> {
///         NoResponse
///     }
/// }
/// ```
pub trait AtatCmd {
    /// The max length of the command.
    ///
    /// Example: For the command "AT+RST" you would specify
    ///
    /// ```
    /// type CommandLen = heapless::consts::U6;
    /// ```
    type CommandLen: ArrayLength<u8>;

    /// The type of the response. Must implement the `AtatResp` trait.
    type Response: AtatResp;

    /// Return the command as a heapless `Vec` of bytes.
    fn as_bytes(&self) -> Vec<u8, Self::CommandLen>;

    /// Parse the string response into a `Self::Response` instance.
    fn parse(&self, resp: &[u8]) -> Result<Self::Response, Error>;

    /// Whether or not this command can be aborted.
    fn can_abort(&self) -> bool {
        false
    }

    /// The max timeout in milliseconds.
    fn max_timeout_ms(&self) -> u32 {
        1000
    }

    /// Force the ingress manager into receive state immediately after sending
    /// the command.
    fn force_receive_state(&self) -> bool {
        false
    }
}

pub trait AtatClient {
    /// Send an AT command.
    ///
    /// `cmd` must implement [`AtatCmd`].
    ///
    /// This function will block until a response is received, if in Timeout or
    /// Blocking mode. In Nonblocking mode, the send can be called until it no
    /// longer returns nb::Error::WouldBlock, or `self.check_response(cmd)` can
    /// be called, with the same result.
    ///
    /// This function will also make sure that atleast `self.config.cmd_cooldown`
    /// has passed since the last response or URC has been received, to allow
    /// the slave AT device time to deliver URC's.
    fn send<A: AtatCmd>(&mut self, cmd: &A) -> nb::Result<A::Response, Error>;

    /// Checks if there are any URC's (Unsolicited Response Code) in
    /// queue from the ingress manager.
    ///
    /// Example:
    /// ```
    /// /// use atat::prelude::*;
    ///
    /// #[derive(Clone, AtatResp)]
    /// pub struct MessageWaitingIndication {
    ///     #[at_arg(position = 0)]
    ///     pub status: u8,
    ///     #[at_arg(position = 1)]
    ///     pub code: u8,
    /// }
    ///
    /// #[derive(Clone, AtatUrc)]
    /// pub enum Urc {
    ///     #[at_urc("+UMWI")]
    ///     MessageWaitingIndication(MessageWaitingIndication),
    /// }
    ///
    /// match client.check_urc::<Urc>() {
    ///     Some(Urc::MessageWaitingIndication(MessageWaitingIndication { status, code })) => {
    ///         // Do something to act on `+UMWI` URC
    ///     }
    /// }
    /// ```
    fn check_urc<URC: AtatUrc>(&mut self) -> Option<URC::Response>;

    /// Check if there are any responses enqueued from the ingress manager.
    ///
    /// The function will return `nb::Error::WouldBlock` until a response or an
    /// error is available, or a timeout occurs and `config.mode` is Timeout.
    ///
    /// This function is usually only called through [`send`].
    ///
    /// [`send`]: #method.send
    fn check_response<A: AtatCmd>(&mut self, cmd: &A) -> nb::Result<A::Response, Error>;

    /// Get the configured mode of the client.
    ///
    /// Options are:
    /// - NonBlocking
    /// - Blocking
    /// - Timeout
    fn get_mode(&self) -> Mode;
}
