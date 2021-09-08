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
        streaming::{alpha0, crlf, multispace1, not_line_ending},
        streaming::{alpha1, alphanumeric0, alphanumeric1, line_ending, none_of, one_of, space1},
    },
    combinator::{not, opt, recognize},
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
#[derive(Debug, Default)]
pub struct NomDigester {
    /// Current processing state.
    state: State,
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
        let preceeding_ws = ws.len();

        // First parse the optional echo and discard it
        let (buf, echo_bytes) = match opt(echo)(buf) {
            Ok((buf, echo)) => (buf, echo.unwrap_or_default().len()),
            Err(nom::Err::Incomplete(_)) => return (DigestResult::None, 0),
            Err(e) => panic!("NOM ERROR {:?}", e),
        };

        // At this point we are ready to look for an actual command response or a URC

        // TODO: Change this to alt parsing
        match opt(response)(buf) {
            Ok((buf, Some((response, len)))) => {
                return (
                    DigestResult::Response(Ok(response)),
                    len + echo_bytes + preceeding_ws,
                )
            }
            Ok((buf, None)) => (DigestResult::None, echo_bytes + preceeding_ws),
            Err(nom::Err::Incomplete(_)) => {
                return (DigestResult::None, echo_bytes + preceeding_ws)
            }
            Err(e) => {
                panic!("NOM ERROR {:?}", e)
            }
        };

        match opt(urc)(buf) {
            Ok((buf, Some((urc, len)))) => {
                return (DigestResult::Urc(urc), len + echo_bytes + preceeding_ws)
            }
            Ok((buf, None)) => (DigestResult::None, echo_bytes + preceeding_ws),
            Err(nom::Err::Incomplete(_)) => (DigestResult::None, echo_bytes + preceeding_ws),
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
        terminated(tag_no_case("at"), not(space1)),
        opt(alt((
            tuple((command, alt((tag("?"), tag("=?"))))),
            tuple((command, arguments)),
        ))),
    )))(i)
}

/// Matches all arguments until `\r\n`. Eg `=3,16\r\n`
fn arguments(i: &[u8]) -> IResult<&[u8], &[u8]> {
    preceded(opt(tag("=")), take_until("\r\n"))(i)
}

/// Matches all parameters until `\r\nOK\r\n`
fn parameters(i: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        multispace0,
        alt((take_until("\r\nOK\r\n"), take_until("\r\nERROR\r\n"))),
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
fn response_code(i: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        multispace0,
        alt((tag_no_case("OK"), tag_no_case("ERROR"))),
    )))(i)
}

// Matches a full AT URC.
fn urc(i: &[u8]) -> IResult<&[u8], (&[u8], usize)> {
    let (i, urc) = recognize(tuple((command, take_until("\r\n"))))(i)?;
    if i.is_empty() {
        return Err(nom::Err::Incomplete(nom::Needed::new(1)));
    }
    // Make sure this is not actually a response
    let (i, _) = not(response_code)(i)?;

    Ok((i, (urc, urc.len())))
}

/// Matches a full AT response.
fn response(i: &[u8]) -> IResult<&[u8], (&[u8], usize)> {
    let (i, ws) = multispace0(i)?;
    let (i, maybe_response) = opt(cmd_parameters)(i)?;
    let response = maybe_response.unwrap_or_default();
    let (i, response_code) = response_code(i)?;

    Ok((
        i,
        (response, response.len() + response_code.len() + ws.len()),
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
        assert_eq!(r, &b"\r\n"[..]);
        assert_eq!(e.len(), 2);

        let (r, e) = echo(b"AT+GMR\r\r\n").unwrap();
        assert_eq!(r, &b"\r\n"[..]);
        assert_eq!(e.len(), 7);

        let (r, e) = echo(b"AT\r\r\n\r\n").unwrap();
        assert_eq!(r, &b"\r\r\n\r\n"[..]);
        assert_eq!(e.len(), 2);

        let (r, e) = echo(b"AT+USORD=3,16\r\n").unwrap();
        assert_eq!(r, &b"\r\n"[..]);
        assert_eq!(e.len(), 13);

        let (r, e) = echo(b"AT+CMUX=?\r\n").unwrap();
        assert_eq!(r, &b"\r\n"[..]);
        assert_eq!(e.len(), 9);

        let (r, e) = echo(b"AT+CMUX?\r\n").unwrap();
        assert_eq!(r, &b"\r\n"[..]);
        assert_eq!(e.len(), 8);
    }

    #[test]
    fn urc_test() {
        let (r, (e, l)) = urc(b"+UUSORD: 3,16,\"16 bytes of data\"\r\n").unwrap();
        assert_eq!(r, &b"\r\n"[..]);
        assert_eq!(e, &b"+UUSORD: 3,16,\"16 bytes of data\""[..]);
        assert_eq!(l, 34);

        assert_eq!(
            urc(b"+UUSORD: 3,16,\"16 bytes of data\"\r\nOK"),
            Err(nom::Err::Incomplete(Needed::new(1)))
        );
        assert_eq!(
            urc(b"+UUSORD: 3,16,\"16 bytes of data\"\r\nERR"),
            Err(nom::Err::Incomplete(Needed::new(2)))
        );

        let (r, (e, l)) = urc(b"+UUSORD: 3,16,\"16 bytes of data\"\r\nOK").unwrap();
        assert_eq!(r, &b"OK"[..]);
        assert_eq!(e, &b"+UUSORD: 3,16,\"16 bytes of data\""[..]);
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
    fn no_response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT\r\r\n\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 2));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"OK\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::Response(Ok(&[])), 7));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert_eq!(&buf, b"\r\n");
    }

    #[test]
    fn response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 13));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!((res, bytes), (DigestResult::None, 2));

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

        assert_eq!(&buf, b"\r\n");
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

        assert_eq!(&buf, b"\r\n");
    }

    #[test]
    fn multi_line_response() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+GMR\r\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 7));
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
        assert_eq!(&buf, b"\r\n");
    }

    // #[test]
    // fn urc() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"+UUSORD: 3,16,\"16 bytes of data\"\r\n")
    //         .unwrap();
    //     let result = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    //     {
    //         let expectation =
    //             Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+UUSORD: 3,16,\"16 bytes of data\"")
    //                 .unwrap();
    //         assert_eq!(result, DigestResult::Urc(expectation));
    //     }
    // }

    // TODO: What does this actually test?
    #[test]
    fn read_error() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());

        buf.extend_from_slice(b"OK\r\n").unwrap();

        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::Response(Ok(&[])), 2));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert_eq!(&buf, b"\r\n");
    }

    #[test]
    #[ignore]
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
                0
            )
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
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
        assert_eq!((res, bytes), (DigestResult::None, 2));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        // buf.extend_from_slice(b"\n").unwrap();
        // let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        // assert_eq!((res, bytes), (DigestResult::None, 2));
        // buf.rotate_left(bytes);
        // buf.truncate(buf.len() - bytes);
    }

    // /// If an invalid response ends with a line terminator, the incomplete flag
    // /// should be cleared.
    // #[test]
    // fn invalid_line_with_termination() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"some status msg\r\n").unwrap();
    //     let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!((res, bytes), (DigestResult::None, 0));
    //     buf.rotate_left(bytes);
    //     buf.truncate(buf.len() - bytes);

    //     buf.extend_from_slice(b"AT+GMR\r\r\n").unwrap();
    //     let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!((res, bytes), (DigestResult::None, 9));
    //     buf.rotate_left(bytes);
    //     buf.truncate(buf.len() - bytes);
    // }

    // /// If a valid response follows an invalid response, the buffer should not
    // /// be cleared in between.
    // #[test]
    // fn mixed_response() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"some status msg\r\nAT+GMR\r\r\n")
    //         .unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );
    // }

    // #[test]
    // fn clear_buf_complete() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"hello\r\ngoodbye\r\n").unwrap();
    //     assert_eq!(
    //         buf,
    //         Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"hello\r\ngoodbye\r\n").unwrap()
    //     );

    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );
    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"").unwrap());
    // }

    // #[test]
    // fn clear_buf_partial() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"hello\r\nthere\r\ngoodbye\r\n")
    //         .unwrap();
    //     assert_eq!(
    //         buf,
    //         Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"hello\r\nthere\r\ngoodbye\r\n").unwrap()
    //     );

    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     assert_eq!(
    //         buf,
    //         Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"there\r\ngoodbye\r\n").unwrap()
    //     );

    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     assert_eq!(
    //         buf,
    //         Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"goodbye\r\n").unwrap()
    //     );

    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );
    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"").unwrap());
    // }

    // #[test]
    // fn clear_buf_partial_no_newlines() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"no newlines anywhere").unwrap();

    //     assert_eq!(
    //         buf,
    //         Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"no newlines anywhere").unwrap()
    //     );

    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"").unwrap());
    // }

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

    // #[test]
    // fn numeric_error_response() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
    //         .unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     buf.extend_from_slice(b"+CME ERROR: 123\r\n").unwrap();
    //     let result = digester.digest(&mut buf, &mut urc_matcher);

    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    //     assert_eq!(
    //         result,
    //         DigestResult::Response(Err(InternalError::Error(
    //             Vec::from_slice(b"+CME ERROR: 123").unwrap()
    //         )))
    //     );
    // }

    // #[test]
    // fn verbose_error_response() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
    //         .unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     buf.extend_from_slice(b"+CME ERROR: Operation not allowed\r\n")
    //         .unwrap();
    //     let result = digester.digest(&mut buf, &mut urc_matcher);

    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    //     assert_eq!(
    //         result,
    //         DigestResult::Response(Err(InternalError::Error(
    //             Vec::from_slice(b"+CME ERROR: Operation not allowed").unwrap()
    //         )))
    //     );
    // }

    // #[test]
    // fn truncate_verbose_error_response() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
    //         .unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     buf.extend_from_slice(b"+CME ERROR: Operation not allowed.. This is a very long error message, that will never fit in my buffer!\r\n").unwrap();
    //     let result = digester.digest(&mut buf, &mut urc_matcher);

    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    //     assert_eq!(
    //         result,
    //         DigestResult::Response(Err(InternalError::Error(
    //             Vec::from_slice(
    //                 b"+CME ERROR: Operation not allowed.. This is a very long error message, that will neve"
    //             )
    //             .unwrap()
    //         )))
    //     );
    // }

    // #[test]
    // fn data_ready_prompt() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"AT+USECMNG=0,0,\"Verisign\",1758\r>")
    //         .unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     let result = digester.digest(&mut buf, &mut urc_matcher);

    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    //     assert_eq!(result, DigestResult::Response(Ok(heapless::Vec::new())));
    // }

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
            (DigestResult::Response(Ok(b"+CPIN: READY\r\n")), 29)
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
    }

    // // Regression test for #87
    // #[test]
    // fn cpin_error() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"AT+CPIN?\r\r\n+CME ERROR: 10\r\n")
    //         .unwrap();

    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );
    //     assert_eq!(
    //         buf,
    //         Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+CME ERROR: 10\r\n").unwrap()
    //     );

    //     let result = digester.digest(&mut buf, &mut urc_matcher);

    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    //     assert_eq!(
    //         result,
    //         DigestResult::Response(Err(InternalError::Error(
    //             Vec::from_slice(b"+CME ERROR: 10").unwrap()
    //         )))
    //     );
    // }

    #[test]
    fn multi_line_response_with_ok() {
        let mut digester = NomDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, 1024>::new();

        buf.extend_from_slice(b"AT+URDBLOCK=\"response.txt\",0,512\r\r\n+")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!((res, bytes), (DigestResult::None, 33));
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

        assert_eq!(&buf, b"\r\n");
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
            (DigestResult::Response(Ok(b"+CPIN: READY\r\n")), 29)
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

        assert_eq!(&buf, b"\r\n");
    }
}
