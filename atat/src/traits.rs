use crate::error::{Error, InternalError};
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
    type Response: Clone;

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
    const ATTEMPTS: u8 = 1;

    /// Whether or not to reattempt a command on a parse error
    /// using `send_retry`.
    const REATTEMPT_ON_PARSE_ERR: bool = true;

    /// Force client to look for a response.
    /// Empty slice is then passed to parse by client.
    /// Implemented to enhance expandability of ATAT
    const EXPECTS_RESPONSE_CODE: bool = true;

    /// Return the command as a heapless `Vec` of bytes.
    fn as_bytes(&self) -> Vec<u8, LEN>;

    fn get_slice<'a>(&'a self, bytes: &'a Vec<u8, LEN>) -> &'a [u8] {
        bytes
    }

    /// Parse the response into a `Self::Response` or `Error` instance.
    fn parse(&self, resp: Result<&[u8], InternalError>) -> Result<Self::Response, Error>;
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
        String::try_from(utf8_string).map_err(|_| Error::Parse)
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
                pdp_type: String::try_from("IP").unwrap(),
                apn: String::try_from("em").unwrap(),
                pdp_addr: String::try_from("100.92.188.66").unwrap(),
                d_comp: 0,
                h_comp: 0,
                ipv4_addr_alloc: Some(0),
                emergency_indication: Some(0),
                p_cscf_discovery: Some(0),
                im_cn_signalling_flag_ind: Some(0),
            },
            PDPContextDefinition {
                cid: 1,
                pdp_type: String::try_from("IP").unwrap(),
                apn: String::try_from("STATREAL").unwrap(),
                pdp_addr: String::try_from("0.0.0.0").unwrap(),
                d_comp: 0,
                h_comp: 0,
                ipv4_addr_alloc: None,
                emergency_indication: None,
                p_cscf_discovery: None,
                im_cn_signalling_flag_ind: None,
            },
            PDPContextDefinition {
                cid: 3,
                pdp_type: String::try_from("IP").unwrap(),
                apn: String::try_from("tim.ibox.it").unwrap(),
                pdp_addr: String::try_from("0.0.0.0").unwrap(),
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
