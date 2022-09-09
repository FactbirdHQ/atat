use crate::error::{Error, InternalError};
use crate::Mode;
use heapless::{String, Vec};

/// This trait needs to be implemented for every response type.
///
/// Example:
/// ```
/// use atat::AtatResp;
///
/// pub struct GreetingText {
///     pub text: heapless::String<64>,
/// }
///
/// impl AtatResp for GreetingText {}
/// ```
pub trait AtatResp {}

pub trait AtatUrc {
    /// The type of the response. Usually the enum this trait is implemented on.
    type Response;

    /// Parse the response into a `Self::Response` instance.
    fn parse(resp: &[u8]) -> Option<Self::Response>;
}

/// This trait needs to be implemented for every command type.
///
/// It can also be derived by the [`atat_derive`] crate.
///
/// [`atat_derive`]: https://crates.io/crates/atat_derive
///
/// Example:
/// ```
/// use atat::{AtatCmd, AtatResp, Error, InternalError};
/// use core::fmt::Write;
/// use heapless::Vec;
///
/// pub struct SetGreetingText<'a> {
///     pub text: &'a str,
/// }
///
/// pub struct NoResponse;
///
/// impl AtatResp for NoResponse {};
///
/// impl<'a> AtatCmd<64> for SetGreetingText<'a> {
///     type Response = NoResponse;
///
///     fn as_bytes(&self) -> Vec<u8, 64> {
///         let mut buf: Vec<u8, 64> = Vec::new();
///         write!(buf, "AT+CSGT={}", self.text);
///         buf
///     }
///
///     fn parse(&self, resp: Result<&[u8], InternalError>) -> Result<Self::Response, Error> {
///         Ok(NoResponse)
///     }
/// }
/// ```
pub trait AtatCmd<const LEN: usize> {
    /// The type of the response. Must implement the `AtatResp` trait.
    type Response: AtatResp;

    /// Whether or not this command can be aborted.
    const CAN_ABORT: bool = false;

    /// The max timeout in milliseconds.
    const MAX_TIMEOUT_MS: u32 = 1000;

    /// The max number of times to attempt a command with automatic retries if
    /// using `send_retry`.
    const ATTEMPTS: u8 = 3;

    /// Force client to look for a response.
    /// Empty slice is then passed to parse by client.
    /// Implemented to enhance expandability fo ATAT
    const EXPECTS_RESPONSE_CODE: bool = true;

    /// Return the command as a heapless `Vec` of bytes.
    fn as_bytes(&self) -> Vec<u8, LEN>;

    /// Parse the response into a `Self::Response` or `Error` instance.
    fn parse(&self, resp: Result<&[u8], InternalError>) -> Result<Self::Response, Error>;
}

pub trait AtatClient {
    /// Send an AT command.
    ///
    /// `cmd` must implement [`AtatCmd`].
    ///
    /// This function will block until a response is received, if in Timeout or
    /// Blocking mode. In Nonblocking mode, the send can be called until it no
    /// longer returns `nb::Error::WouldBlock`, or `self.check_response(cmd)` can
    /// be called, with the same result.
    ///
    /// This function will also make sure that atleast `self.config.cmd_cooldown`
    /// has passed since the last response or URC has been received, to allow
    /// the slave AT device time to deliver URC's.
    fn send<A: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &A,
    ) -> nb::Result<A::Response, Error>;

    fn send_retry<A: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &A,
    ) -> nb::Result<A::Response, Error> {
        let mut error = Err(nb::Error::Other(Error::Error));

        for attempt in 1..=A::ATTEMPTS {
            if attempt > 1 {
                debug!("Attempt {}:", attempt);
            }

            match self.send(cmd) {
                e @ Err(nb::Error::Other(Error::Timeout | Error::Parse)) => {
                    error = e;
                }
                r => return r,
            }
        }
        error
    }

    /// Checks if there are any URC's (Unsolicited Response Code) in
    /// queue from the ingress manager.
    ///
    /// Example:
    /// ```
    /// use atat::atat_derive::{AtatResp, AtatUrc};
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
    /// // match client.check_urc::<Urc>() {
    /// //     Some(Urc::MessageWaitingIndication(MessageWaitingIndication { status, code })) => {
    /// //         // Do something to act on `+UMWI` URC
    /// //     }
    /// // }
    /// ```
    fn check_urc<URC: AtatUrc>(&mut self) -> Option<URC::Response> {
        let mut return_urc = None;
        self.peek_urc_with::<URC, _>(|urc| {
            return_urc = Some(urc);
            true
        });
        return_urc
    }

    fn peek_urc_with<URC: AtatUrc, F: FnOnce(URC::Response) -> bool>(&mut self, f: F);

    /// Check if there are any responses enqueued from the ingress manager.
    ///
    /// The function will return `nb::Error::WouldBlock` until a response or an
    /// error is available, or a timeout occurs and `config.mode` is Timeout.
    ///
    /// This function is usually only called through [`send`].
    ///
    /// [`send`]: #method.send
    fn check_response<A: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &A,
    ) -> nb::Result<A::Response, Error>;

    /// Get the configured mode of the client.
    ///
    /// Options are:
    /// - `NonBlocking`
    /// - `Blocking`
    /// - `Timeout`
    fn get_mode(&self) -> Mode;

    /// Reset the client, queues and ingress buffer, discarding any contents
    fn reset(&mut self);
}

impl<T, const L: usize> AtatResp for Vec<T, L> where T: AtatResp {}

impl<const L: usize> AtatResp for String<L> {}

impl<const L: usize> AtatCmd<L> for String<L> {
    type Response = String<256>;

    fn as_bytes(&self) -> Vec<u8, L> {
        self.clone().into_bytes()
    }

    fn parse(&self, resp: Result<&[u8], InternalError>) -> Result<Self::Response, Error> {
        let utf8_string =
            core::str::from_utf8(resp.map_err(Error::from)?).map_err(|_| Error::Parse)?;
        Ok(String::from(utf8_string))
    }
}

#[cfg(all(test, feature = "derive"))]
mod test {
    use super::*;
    use crate as atat;
    use atat_derive::{AtatEnum, AtatResp};
    use heapless::String;

    #[derive(Debug, Clone, PartialEq, AtatEnum)]
    pub enum PDPContextStatus {
        /// 0: deactivated
        Deactivated = 0,
        /// 1: activated
        Activated = 1,
    }

    #[derive(Debug, Clone, AtatResp, PartialEq)]
    pub struct PDPContextState {
        #[at_arg(position = 0)]
        pub cid: u8,
        #[at_arg(position = 1)]
        pub status: PDPContextStatus,
    }

    #[derive(Debug, Clone, AtatResp, PartialEq)]
    pub struct PDPContextDefinition {
        #[at_arg(position = 0)]
        pub cid: u8,
        #[at_arg(position = 1)]
        pub pdp_type: String<6>,
        #[at_arg(position = 2)]
        pub apn: String<99>,
        #[at_arg(position = 3)]
        pub pdp_addr: String<99>,
        #[at_arg(position = 4)]
        pub d_comp: u8,
        #[at_arg(position = 5)]
        pub h_comp: u8,
        #[at_arg(position = 6)]
        pub ipv4_addr_alloc: Option<u8>,
        #[at_arg(position = 7)]
        pub emergency_indication: Option<u8>,
        #[at_arg(position = 8)]
        pub p_cscf_discovery: Option<u8>,
        #[at_arg(position = 9)]
        pub im_cn_signalling_flag_ind: Option<u8>,
        /* #[at_arg(position = 10)]
         * pub nslpi: Option<u8>, */
    }

    #[test]
    fn single_multi_response() {
        let mut v = Vec::<_, 1>::from_slice(&[PDPContextState {
            cid: 1,
            status: PDPContextStatus::Deactivated,
        }])
        .unwrap();

        let mut resp: Vec<PDPContextState, 1> = serde_at::from_slice(b"+CGACT: 1,0\r\n").unwrap();

        assert_eq!(resp.pop(), v.pop());
        assert_eq!(resp.pop(), None);
    }

    #[test]
    fn multi_response() {
        let mut v = Vec::<_, 3>::from_slice(&[
            PDPContextState {
                cid: 1,
                status: PDPContextStatus::Deactivated,
            },
            PDPContextState {
                cid: 2,
                status: PDPContextStatus::Activated,
            },
            PDPContextState {
                cid: 3,
                status: PDPContextStatus::Deactivated,
            },
        ])
        .unwrap();

        let mut resp: Vec<PDPContextState, 3> =
            serde_at::from_slice(b"+CGACT: 1,0\r\n+CGACT: 2,1\r\n+CGACT: 3,0").unwrap();

        assert_eq!(resp.pop(), v.pop());
        assert_eq!(resp.pop(), v.pop());
        assert_eq!(resp.pop(), v.pop());
        assert_eq!(resp.pop(), None);
    }

    #[test]
    fn multi_response_advanced() {
        let mut v = Vec::<_, 3>::from_slice(&[
            PDPContextDefinition {
                cid: 2,
                pdp_type: String::from("IP"),
                apn: String::from("em"),
                pdp_addr: String::from("100.92.188.66"),
                d_comp: 0,
                h_comp: 0,
                ipv4_addr_alloc: Some(0),
                emergency_indication: Some(0),
                p_cscf_discovery: Some(0),
                im_cn_signalling_flag_ind: Some(0),
            },
            PDPContextDefinition {
                cid: 1,
                pdp_type: String::from("IP"),
                apn: String::from("STATREAL"),
                pdp_addr: String::from("0.0.0.0"),
                d_comp: 0,
                h_comp: 0,
                ipv4_addr_alloc: None,
                emergency_indication: None,
                p_cscf_discovery: None,
                im_cn_signalling_flag_ind: None,
            },
            PDPContextDefinition {
                cid: 3,
                pdp_type: String::from("IP"),
                apn: String::from("tim.ibox.it"),
                pdp_addr: String::from("0.0.0.0"),
                d_comp: 0,
                h_comp: 0,
                ipv4_addr_alloc: None,
                emergency_indication: None,
                p_cscf_discovery: None,
                im_cn_signalling_flag_ind: None,
            },
        ])
        .unwrap();

        let mut resp: Vec<PDPContextDefinition, 3> =
            serde_at::from_slice(b"+CGDCONT: 2,\"IP\",\"em\",\"100.92.188.66\",0,0,0,0,0,0\r\n+CGDCONT: 1,\"IP\",\"STATREAL\",\"0.0.0.0\",0,0\r\n+CGDCONT: 3,\"IP\",\"tim.ibox.it\",\"0.0.0.0\",0,0").unwrap();

        assert_eq!(resp.pop(), v.pop());
        assert_eq!(resp.pop(), v.pop());
        assert_eq!(resp.pop(), v.pop());
        assert_eq!(resp.pop(), None);
    }
}
