use crate::{
    atat_log,
    helpers::LossyStr,
    urc_matcher::{UrcMatcher, UrcMatcherResult},
    Digester, InternalError,
};
use heapless::Vec;
use nom::{
    branch::alt,
    bytes::streaming::{tag, take, take_till, take_until, take_while, take_while1},
    character::{
        complete,
        streaming::{alpha0, crlf, multispace1, not_line_ending},
        streaming::{alpha1, alphanumeric0, alphanumeric1, line_ending, none_of, one_of, space1},
    },
    combinator::{self, eof, map, not, opt, peek, recognize},
    error::dbg_dmp,
    multi::many0_count,
    sequence::{delimited, separated_pair, tuple},
    AsChar,
};
use nom::{
    bytes::streaming::tag_no_case,
    character::streaming::multispace0,
    sequence::{preceded, terminated},
    IResult,
};

/// State of the `NomDigester`, used to distiguish URCs from solicited
/// responses
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum State {
    Idle,
    ReceivingResponse,
}

impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, PartialEq)]
pub enum DigestResult<'a> {
    Urc(&'a [u8]),
    Response(Result<&'a [u8], InternalError>),
    Prompt,
    None,
}

pub trait NewDigester {
    /// Command line termination character S3 (Default = b'\r' ASCII: \[013\])
    const LINE_TERM_CHAR: u8 = b'\r';

    /// Response formatting character S4 (Default = b'\n' ASCII: \[010\])
    const FORMAT_CHAR: u8 = b'\n';

    fn reset(&mut self);

    fn force_receive_state(&mut self);

    fn digest<'a>(
        &mut self,
        buf: &'a [u8],
        urc_matcher: &mut impl UrcMatcher,
    ) -> (DigestResult<'a>, usize);
}

/// A Digester that tries to implement the basic AT standard.
/// This digester should work for most usecases of ATAT.
///
/// Implements a request-response AT digester capable of working with or without AT echo enabled.
///
/// Buffer can contain ('...' meaning arbitrary data):
/// - '...AT<CMD>\r\r\n<RESPONSE>\r\n<RESPONSE CODE>\r\n...'             (Echo enabled)
/// - '...AT<CMD>\r\r\n<CMD>:<PARAMETERS>\r\n<RESPONSE CODE>\r\n...'     (Echo enabled)
/// - '...AT<CMD>\r\r\n<RESPONSE CODE>\r\n...'                           (Echo enabled)
/// - '...<CMD>:<PARAMETERS>\r\n<RESPONSE CODE>\r\n...'                  (Echo disabled)
/// - '...<RESPONSE>\r\n<RESPONSE CODE>\r\n...'                          (Echo disabled)
/// - '...<URC>\r\n...'                                                  (Unsolicited response code)
/// - '...<URC>:<PARAMETERS>\r\n...'                                     (Unsolicited response code)
/// - '...<PROMPT>\r\n'                                                  (Prompt for data)
///
/// Goal of the digester is to extract these into:
/// - DigestResult::Response(Result<RESPONSE>)
/// - DigestResult::Urc(<URC>)
/// - DigestResult::Prompt
/// - DigestResult::None
///
/// Usually <RESPONSE CODE> is one of ['OK', 'ERROR', 'CME ERROR: <NUMBER/STRING>', 'CMS ERROR: <NUMBER/STRING>'],
/// but can be others as well depending on manufacturer.
///
/// Usually <PROMPT> can be one of ['>', '@'], and is command specific and only valid for few selected commands.
///
/// **Limitations**:
/// - URC's cannot be distingushed from responses until there is at least
///   one more char in the buffer, not matching any valid response code.
///   Eg `<URC>:<PARAMETERS>\r\nA` would be parsed as a URC, while
///   `<URC>:<PARAMETERS>\r\n` is impossible to distingush from
///   `<CMD>:<PARAMETERS>\r\n` until we gain more data.
#[derive(Debug)]
pub struct NomDigester {
    /// Current processing state.
    state: State,
    prompts: &'static [u8],
}

impl NomDigester {
    pub fn new() -> Self {
        Self {
            state: Default::default(),
            prompts: &[b'>'],
        }
    }
}

impl Default for NomDigester {
    fn default() -> Self {
        Self::new()
    }
}

impl NewDigester for NomDigester {
    fn reset(&mut self) {
        self.state = State::Idle;
    }

    fn force_receive_state(&mut self) {
        self.state = State::ReceivingResponse;
    }

    fn digest<'a>(
        &mut self,
        buf: &'a [u8],
        urc_matcher: &mut impl UrcMatcher,
    ) -> (DigestResult<'a>, usize) {
        // Trim any leading whitespace
        let (buf, ws) = multispace0::<&[u8], nom::error::Error<&[u8]>>(buf).unwrap();

        // First parse the optional echo and discard it
        let (buf, echo_bytes) = match opt(echo)(buf) {
            Ok((buf, echo)) => (buf, echo.unwrap_or_default().len()),
            Err(nom::Err::Incomplete(_)) => return (DigestResult::None, 0),
            Err(e) => panic!("NOM ERROR {:?}", e),
        };

        // TODO: Change to prompt fn & extend with more prompts? Also add way for custom prompts
        if let Ok((_, Some(p))) = opt::<_, _, nom::error::Error<&[u8]>, _>(tag(self.prompts))(buf) {
            return (DigestResult::Prompt, echo_bytes + p.len());
        }

        // At this point we are ready to look for an actual command response or a URC
        match alt((response, urc))(buf) {
            Ok((buf, (response, len))) => (response, len + echo_bytes + ws.len()),
            Err(nom::Err::Incomplete(_)) => (DigestResult::None, echo_bytes + ws.len()),
            Err(e) => {
                panic!("NOM ERROR {:?}", e)
            }
        }
    }
}

fn print_dbg(i: &[u8]) -> IResult<&[u8], &[u8]> {
    dbg!(core::str::from_utf8(i).unwrap());
    Ok((i, &[]))
}

/// Matches a full AT echo. Eg `AT+USORD=3,16\r\n`
fn echo(i: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        terminated(tag_no_case("AT"), not(space1)),
        opt(alt((
            tuple((command, alt((tag("?"), tag("=?"))))),
            tuple((command, preceded(opt(tag("=")), take_until("\r")))),
        ))),
        complete::multispace0,
    )))(i)
}

/// Matches all parameters until `\r\nOK\r\n`
/// TODO: Should this be quote pair aware, to handle the eg. `OK` in a received string?
fn parameters(i: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(alt((
        nom::bytes::complete::take_until("\r\nOK\r\n"),
        nom::bytes::complete::take_until("\r\nERROR\r\n"),
        nom::bytes::complete::take_until("+CME ERROR"),
    )))(i)
}

/// Matches a single AT command. Eg `+USORD`
fn command(i: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        opt(alt((tag("+"), tag("&"), tag("\\")))),
        alphanumeric1,
    )))(i)
}

/// Matches a single AT command plus parameters up till, but not including
/// response code. Eg `+USORD: 3,16,123`
fn cmd_parameters(i: &[u8]) -> IResult<&[u8], &[u8]> {
    // Make sure we don't accidentally eat a response code as command
    not(response_code)(i)?;
    recognize(tuple((opt(tuple((command, tag(":")))), parameters)))(i)
}

/// Matches a valid AT response code, including leading & trailing
/// whitespace/newlines
fn response_code(i: &[u8]) -> IResult<&[u8], (Result<&[u8], InternalError>, usize)> {
    let (i, ws) = multispace0(i)?;
    let (i, (code, len)) = alt((
        map(tag_no_case("OK\r\n"), |s: &[u8]| (Ok(s), s.len())),
        map(
            tuple((tag_no_case("ERROR"), complete::multispace1)),
            |(s, le): (&[u8], &[u8])| (Err(s.into()), s.len() + le.len()),
        ),
        // TODO: Generalize for CMS, CME, and other custom errors somehow?
        map(
            tuple((
                tag_no_case("+CME ERROR"),
                take_until("\r\n"),
                complete::multispace0,
            )),
            |(tag, code, le): (_, &[u8], _)| {
                (Err((tag, code).into()), tag.len() + code.len() + le.len())
            },
        ),
    ))(i)?;

    Ok((i, (code, len + ws.len())))
}

/// Matches a full AT URC.
fn urc(i: &[u8]) -> IResult<&[u8], (DigestResult, usize)> {
    let (i, ws_prec) = multispace0(i)?;
    let (i, urc) = recognize(tuple((command, take_until("\r\n"))))(i)?;
    let (i, ws) = multispace0(i)?;

    let (i, _) = alt((eof, recognize(not(response_code))))(i)?;

    Ok((
        i,
        (DigestResult::Urc(urc), ws_prec.len() + urc.len() + ws.len()),
    ))
}

/// Matches a full AT response.
fn response(i: &[u8]) -> IResult<&[u8], (DigestResult, usize)> {
    let (i, ws) = multispace0(i)?;
    let (i, maybe_response) = opt(cmd_parameters)(i)?;
    let response = maybe_response.unwrap_or_default();
    let (i, (response_code, response_code_len)) = response_code(i)?;

    let response_len = response.len();
    let response = response_code.map(|_| response);

    Ok((
        i,
        (
            DigestResult::Response(response),
            response_len + response_code_len + ws.len(),
        ),
    ))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::helpers::SliceExt;
    use crate::queues::{ComQueue, ResQueue, UrcQueue};
    use crate::urc_matcher::{DefaultUrcMatcher, UrcMatcherResult};
    use heapless::spsc::Queue;
    use nom::Needed;

    const TEST_RX_BUF_LEN: usize = 256;

    #[test]
    fn response_code_test() {
        let (r, e) = response_code(b"OK\r\n").unwrap();
        assert_eq!(r, b"");
        assert!(e.0.is_ok());
        assert_eq!(e.1, 4);

        let (r, e) = response_code(b"ok\r\n").unwrap();
        assert_eq!(r, b"");
        assert!(e.0.is_ok());
        assert_eq!(e.1, 4);

        let (r, e) = response_code(b"ERROR\r\n").unwrap();
        assert_eq!(r, b"");
        assert!(e.0.is_err());
        assert_eq!(e.1, 7);

        let (r, e) = response_code(b"+CME ERROR: 10\r\n").unwrap();
        assert_eq!(r, b"");
        assert_eq!(e.0, Err((&b"+CME ERROR: 10"[..]).into()));
        assert_eq!(e.1, 16);

        let (r, e) = response_code(b"+CME ERROR: This is a verbose error\r\n").unwrap();
        assert_eq!(r, b"");
        assert_eq!(
            e.0,
            Err((&b"+CME ERROR: This is a verbose error"[..]).into())
        );
        assert_eq!(e.1, 37);

        let (r, e) = response_code(b"+CME ERROR: This is a verbose error that is longer than the max allowed error length in InternalError\r\n").unwrap();
        assert_eq!(r, b"");
        assert_eq!(
            e.0,
            Err((&b"+CME ERROR: This is a verbose error that is longer than the max allowed error length "[..]).into())
        );
        assert_eq!(e.1, 103);

        assert_eq!(
            response_code(b"OK"),
            Err(nom::Err::Incomplete(nom::Needed::new(2)))
        );
        assert_eq!(
            response_code(b"ERR"),
            Err(nom::Err::Incomplete(nom::Needed::new(2)))
        );
    }

    #[test]
    fn cmd_test() {
        let r = command(b"+CCID ").unwrap();
        assert_eq!(r, (&b" "[..], &b"+CCID"[..]));

        let r = command(b"+USORD: 3,16,\"16 bytes of data\"\r\n").unwrap();
        assert_eq!(r, (&b": 3,16,\"16 bytes of data\"\r\n"[..], &b"+USORD"[..]));

        let r = command(b"&H ").unwrap();
        assert_eq!(r, (&b" "[..], &b"&H"[..]));

        let r = command(b"\\Q ").unwrap();
        assert_eq!(r, (&b" "[..], &b"\\Q"[..]));

        let r = command(b"S10 ").unwrap();
        assert_eq!(r, (&b" "[..], &b"S10"[..]));

        let r = command(b"I ").unwrap();
        assert_eq!(r, (&b" "[..], &b"I"[..]));
    }

    #[test]
    fn echo_test() {
        let (r, e) = echo(b"AT\r\n").unwrap();
        assert_eq!(r, &b""[..]);
        assert_eq!(e.len(), 4);

        let (r, e) = echo(b"AT+GMR\r\r\n").unwrap();
        assert_eq!(r, &b""[..]);
        assert_eq!(e.len(), 9);

        let (r, e) = echo(b"AT\r\r\n\r\n").unwrap();
        assert_eq!(r, &b""[..]);
        assert_eq!(e.len(), 7);

        let (r, e) = echo(b"AT+USORD=3,16\r\n").unwrap();
        assert_eq!(r, &b""[..]);
        assert_eq!(e.len(), 15);

        let (r, e) = echo(b"AT+CMUX=?\r\n").unwrap();
        assert_eq!(r, &b""[..]);
        assert_eq!(e.len(), 11);

        let (r, e) = echo(b"AT+CMUX?\r\n").unwrap();
        assert_eq!(r, &b""[..]);
        assert_eq!(e.len(), 10);

        let (r, e) = echo(b"AT+CMUX?\r\nAT").unwrap();
        assert_eq!(r, &b"AT"[..]);
        assert_eq!(e.len(), 10);
    }

    #[test]
    fn urc_test() {
        let (r, (e, l)) = urc(b"+UUSORD: 3,16,\"16 bytes of data\"\r\nA").unwrap();
        assert_eq!(r, &b"A"[..]);
        assert_eq!(e, DigestResult::Urc(b"+UUSORD: 3,16,\"16 bytes of data\""));
        assert_eq!(l, 34);

        let (r, (e, l)) = urc(b"+UUNU: 0\r\nA").unwrap();
        assert_eq!(r, &b"A"[..]);
        assert_eq!(e, DigestResult::Urc(b"+UUNU: 0"));
        assert_eq!(l, 10);

        let err = urc(b"+UUSORD: 3,16,\"16 bytes of data\"\r\nOK").unwrap_err();
        assert!(err.is_incomplete());

        let err = urc(b"+UUSORD: 3,16,\"16 bytes of data\"\r\nERR").unwrap_err();
        assert!(err.is_incomplete());

        let (r, (e, l)) = urc(b"+UUSORD: 3,16,\"16 bytes of data\"\r\nERRA").unwrap();
        assert_eq!(r, &b"ERRA"[..]);
        assert_eq!(e, DigestResult::Urc(b"+UUSORD: 3,16,\"16 bytes of data\""));
        assert_eq!(l, 34);

        let err = urc(b"+UUNU: ").unwrap_err();
        assert!(err.is_incomplete());

        let err = urc(b"+UUNU: 0").unwrap_err();
        assert!(err.is_incomplete());

        let err = urc(b"+UUNU: 0\r").unwrap_err();
        assert!(err.is_incomplete());

        let (r, (e, l)) = urc(b"+UUSORD: 3,16,\"16 bytes of data\"\r\nOKA").unwrap();
        assert_eq!(r, &b"OKA"[..]);
        assert_eq!(e, DigestResult::Urc(b"+UUSORD: 3,16,\"16 bytes of data\""));
        assert_eq!(l, 34);

        let (r, (e, l)) = urc(b"+UUSORD: 3,16,\"16 bytes of data\"\r\nAT").unwrap();
        assert_eq!(r, &b"AT"[..]);
        assert_eq!(e, DigestResult::Urc(b"+UUSORD: 3,16,\"16 bytes of data\""));
        assert_eq!(l, 34);
    }

    #[test]
    fn cmd_parameters_test() {
        let (r, e) = cmd_parameters(b"+USORD: 3,16,123\r\nOK\r\n").unwrap();
        assert_eq!(r, &b"\r\nOK\r\n"[..]);
        assert_eq!(e, &b"+USORD: 3,16,123"[..]);
        assert_eq!(e.len(), 16);

        let (r, e) = cmd_parameters(
            b"+UMGC:\r\nPath 0:\r\n\r\n10,9384\r\nPath 1:\r\n12,8192\r\nPath 2:\r\n6,8192\r\nOK\r\n",
        )
        .unwrap();
        assert_eq!(r, &b"\r\nOK\r\n"[..]);
        assert_eq!(
            e,
            &b"+UMGC:\r\nPath 0:\r\n\r\n10,9384\r\nPath 1:\r\n12,8192\r\nPath 2:\r\n6,8192"[..]
        );
        assert_eq!(e.len(), 61);
    }

    #[test]
    fn no_parameters_response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT\r\r\n\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 7));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"OK\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::Response(Ok(&[])), 4));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert_eq!(&buf, b"");
    }

    #[test]
    fn response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 15));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!((res, bytes), (DigestResult::None, 0));

        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        {
            let expectation =
                Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
                    .unwrap();
            assert_eq!(buf, expectation);
        }

        buf.extend_from_slice(b"OK\r\n").unwrap();
        {
            let expectation = Vec::<_, TEST_RX_BUF_LEN>::from_slice(
                b"+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n",
            )
            .unwrap();
            assert_eq!(buf, expectation);
        }
        let (result, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!(
            result,
            DigestResult::Response(Ok(b"+USORD: 3,16,\"16 bytes of data\""))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(&buf, b"");
    }

    #[test]
    fn response_no_echo() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!((res, bytes), (DigestResult::None, 0));

        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        {
            let expectation =
                Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
                    .unwrap();
            assert_eq!(buf, expectation);
        }

        buf.extend_from_slice(b"OK\r\n").unwrap();
        {
            let expectation = Vec::<_, TEST_RX_BUF_LEN>::from_slice(
                b"+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n",
            )
            .unwrap();
            assert_eq!(buf, expectation);
        }
        let (result, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!(
            result,
            DigestResult::Response(Ok(b"+USORD: 3,16,\"16 bytes of data\""))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(&buf, b"");
    }

    #[test]
    fn multi_line_response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+GMR\r\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 9));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"AT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19\r\nOK\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

        {
            let expectation = b"AT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19";
            assert_eq!(res, DigestResult::Response(Ok(expectation)));
        }
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert_eq!(&buf, b"");
    }

    #[test]
    fn urc_digest() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"+UUSORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 0));

        buf.extend_from_slice(b"A").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(
            (res, bytes),
            (DigestResult::Urc(b"+UUSORD: 3,16,\"16 bytes of data\""), 34)
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert_eq!(&buf, b"A");
    }

    // TODO: What does this actually test?
    #[test]
    fn read_error() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());

        buf.extend_from_slice(b"OK\r\n").unwrap();

        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::Response(Ok(&[])), 4));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert_eq!(&buf, b"");
    }

    #[test]
    fn error_response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 15));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"ERROR\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!(
            (res, bytes),
            (
                DigestResult::Response(Err(InternalError::Error(
                    Vec::from_slice(b"ERROR").unwrap()
                ))),
                40
            )
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(buf, b"");
    }

    /// By breaking up non-AT-commands into chunks, it's possible that
    /// they're mistaken for AT commands due to buffer clearing.
    ///
    /// Regression test for #27.
    #[test]
    fn chunkwise_digest() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"THIS FORM").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        // TODO: Does this behavior match the `DefaultDigester`?
        buf.extend_from_slice(b"AT SUCKS\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(buf, b"THIS FORMAT SUCKS\r\n");
    }

    /// By sending AT-commands byte-by-byte, it's possible that
    /// the command is incorrectly ignored due to buffer clearing.
    ///
    /// Regression test for #27.
    #[test]
    fn bytewise_digest() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"A").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"T").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"\r").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 3));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(buf, b"");
    }

    /// If an invalid response ends with a line terminator, it is considered an
    /// URC, as URC's can be all kinds of unknown strings, e.g `RING\r\n`.
    ///
    /// These should be filtered at the client level!
    #[test]
    fn newline_terminated_garbage_becomes_urc() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"some status msg\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"AT+GMR\r\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::Urc(b"some status msg"), 17));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(buf, b"AT+GMR\r\r\n");
    }

    /// If a valid response follows an invalid response, the buffer should not
    /// be cleared in between.
    #[test]
    fn newline_terminated_garbage_becomes_urc_mixed_response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"some status msg\r\nAT+GMR\r\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::Urc(b"some status msg"), 17));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(buf, b"AT+GMR\r\r\n");
    }

    // #[test]
    // fn custom_urc_matcher() {
    //     struct MyUrcMatcher {}
    //     impl UrcMatcher for MyUrcMatcher {
    //         fn process<const L: usize>(&mut self, buf: &mut Vec<u8, L>) -> UrcMatcherResult<L> {
    //             if buf.len() >= 6 && buf.get(0..6) == Some(b"+match") {
    //                 let data = buf.clone();
    //                 buf.truncate(0);
    //                 UrcMatcherResult::Complete(data)
    //             } else if buf.len() >= 4 && buf.get(0..4) == Some(b"+mat") {
    //                 UrcMatcherResult::Incomplete
    //             } else {
    //                 UrcMatcherResult::NotHandled
    //             }
    //         }
    //     }

    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = MyUrcMatcher {};
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     // Initial state

    //     // Check an URC that is not handled by MyUrcMatcher (fall back to default behavior)
    //     // Note that this requires the trailing newlines to be present!
    //     buf.extend_from_slice(b"+default-behavior\r\n").unwrap();
    //     let result = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!(
    //         result,
    //         DigestResult::Urc(Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+default-behavior").unwrap())
    //     );

    //     // Check an URC that is generally handled by MyUrcMatcher but
    //     // considered incomplete (not enough data). This will not yet result in
    //     // an URC being dispatched.
    //     buf.extend_from_slice(b"+mat").unwrap();
    //     let result = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!(result, DigestResult::None);

    //     // Make it complete!
    //     buf.extend_from_slice(b"ch").unwrap(); // Still no newlines, but this will still be picked up.unwrap()!
    //     let result = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!(
    //         result,
    //         DigestResult::Urc(Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+match").unwrap())
    //     );
    // }

    #[test]
    fn numeric_error_response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();

        buf.extend_from_slice(b"+CME ERROR: 123\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(
            res,
            DigestResult::Response(Err(InternalError::Error(
                Vec::from_slice(b"+CME ERROR: 123").unwrap()
            )))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(&buf, b"");
    }

    #[test]
    fn verbose_error_response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();

        buf.extend_from_slice(b"+CME ERROR: Operation not allowed\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(
            res,
            DigestResult::Response(Err(InternalError::Error(
                Vec::from_slice(b"+CME ERROR: Operation not allowed").unwrap()
            )))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(&buf, b"");
    }

    #[test]
    fn truncate_verbose_error_response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        buf.extend_from_slice(b"+CME ERROR: Operation not allowed.. This is a very long error message, that will never fit in my buffer!\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(
            res,
            DigestResult::Response(Err(InternalError::Error(
                Vec::from_slice(
                    b"+CME ERROR: Operation not allowed.. This is a very long error message, that will neve"
                )
                .unwrap()
            )))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(&buf, b"");
    }

    #[test]
    fn data_ready_prompt() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USECMNG=0,0,\"Verisign\",1758\r>")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::Prompt, 32));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
    }

    // Regression test for #87
    #[test]
    fn cpin_parsing() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+CPIN?\r\r\n+CPIN: READY\r\n\r\nOK\r\n")
            .unwrap();

        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!(
            (res, bytes),
            (DigestResult::Response(Ok(b"+CPIN: READY\r\n")), 31)
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert_eq!(&buf, b"");
    }

    // Regression test for #87
    #[test]
    fn cpin_error() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+CPIN?\r\r\n+CME ERROR: 10\r\n")
            .unwrap();

        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(
            res,
            DigestResult::Response(Err(InternalError::Error(
                Vec::from_slice(b"+CME ERROR: 10").unwrap()
            )))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(&buf, b"");
    }

    #[test]
    fn multi_line_response_with_ok() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, 1024>::new();

        buf.extend_from_slice(b"AT+URDBLOCK=\"response.txt\",0,512\r\r\n+")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 35));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"\r\nOK\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        {
            let expectation = Vec::<_, 1024>::from_slice(b"+URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"").unwrap();
            assert_eq!(
                (res, bytes),
                (DigestResult::Response(Ok(&expectation)), 550)
            );
        }
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(&buf, b"");
    }

    #[test]
    fn multi_cmd_multi_line_response_with_ok() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, 2048>::new();

        buf.extend_from_slice(b"AT+CPIN?\r\r\n+CPIN: READY\r\n\r\nOK\r\n")
            .unwrap();

        buf.extend_from_slice(b"AT+URDBLOCK=\"response.txt\",0,512\r\r\n+")
            .unwrap();

        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!(
            (res, bytes),
            (DigestResult::Response(Ok(b"+CPIN: READY\r\n")), 31)
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 35));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"\r\nOK\r\n").unwrap();

        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        {
            let expectation = Vec::<_, 1024>::from_slice(b"+URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"").unwrap();
            assert_eq!(
                (res, bytes),
                (DigestResult::Response(Ok(&expectation)), 550)
            );
        }
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(&buf, b"");
    }
}
