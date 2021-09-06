use crate::{
    atat_log,
    helpers::LossyStr,
    urc_matcher::{UrcMatcher, UrcMatcherResult},
    Digester, InternalError,
};
use heapless::Vec;
use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_till, take_until, take_while},
    character::{
        complete::{alpha0, crlf, multispace1, not_line_ending},
        complete::{alpha1, alphanumeric0, alphanumeric1, line_ending, one_of},
    },
    combinator::{opt, recognize},
    multi::many0_count,
    sequence::{separated_pair, tuple},
};
use nom::{
    bytes::complete::tag_no_case,
    character::complete::multispace0,
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
        // Buffer can contain ('...' meaning arbitrary data):
        // - '...AT<CMD>\r\r\n<RESPONSE>\r\n<RESPONSE CODE>\r\n...'             (Echo enabled)
        // - '...AT<CMD>\r\r\n<CMD>:<PARAMETERS>\r\n<RESPONSE CODE>\r\n...'     (Echo enabled)
        // - '...AT<CMD>\r\r\n<RESPONSE CODE>\r\n...'                           (Echo enabled)
        // - '...<CMD>:<PARAMETERS>\r\n<RESPONSE CODE>\r\n...'                  (Echo disabled)
        // - '...<RESPONSE>\r\n<RESPONSE CODE>\r\n...'                          (Echo disabled)
        // - '...<URC>\r\n...'                                                  (Unsolicited response code)
        // - '...<URC>:<PARAMETERS>\r\n...'                                     (Unsolicited response code)
        // - '...<PROMPT>\r\n'                                                  (Prompt for data)
        //
        // Goal of the digester is to extract these into:
        // - DigestResult::Response(Result<RESPONSE>)
        // - DigestResult::Urc(<URC>)
        // - DigestResult::Prompt
        // - DigestResult::None
        //
        // Usually <RESPONSE CODE> is one of ['OK', 'ERROR', 'CME ERROR: <NUMBER/STRING>', 'CMS ERROR: <NUMBER/STRING>'],
        // but can be others as well depending on manufacturer.
        //
        // Usually <PROMPT> can be one of ['>', '@'], and is command specific and only valid for few selected commands.

        // Trim any
        let (buf, ws) = multispace0::<&[u8], nom::error::Error<&[u8]>>(buf).unwrap();
        let preceeding_ws = ws.len();

        // First parse the optional echo and discard it
        let (buf, echo_bytes) = match opt(echo)(buf) {
            Ok((buf, echo)) => (buf, echo.unwrap_or_default().len()),
            Err(nom::Err::Incomplete(_)) => return (DigestResult::None, 0),
            Err(e) => panic!("NOM ERROR {:?}", e),
        };

        // println!("{:?}", buf);

        match opt(response)(buf) {
            Ok((buf, Some((response, len)))) => {
                (DigestResult::Response(Ok(response)), len + echo_bytes)
            }
            Ok((buf, None)) => (DigestResult::None, echo_bytes),
            Err(nom::Err::Incomplete(_)) => (DigestResult::None, 0),
            Err(e) => {
                panic!("NOM ERROR {:?}", e)
            }
        }
    }
}

fn none(i: &[u8]) -> IResult<&[u8], &[u8]> {
    Ok((i, &[]))
}

fn echo(i: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag_no_case("at"),
        opt(alt((
            tuple((command, alt((tag("?"), tag("=?"))), none)),
            tuple((command, tag("="), parameters)),
        ))),
        multispace1,
    )))(i)
}

// fn at_value(i: &[u8]) -> IResult<&[u8], &[u8]> {}

fn parameters(i: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while(move |c: u8| c.is_ascii_alphanumeric() || c == b',' || c == b'\"')(i)
}

fn command(i: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        opt(alt((tag("+"), tag("&"), tag("\\")))),
        alphanumeric1,
    )))(i)
}

fn cmd_parameters(i: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        command,
        opt(tuple((tag(":"), multispace0, parameters))),
        multispace1,
        response_code,
    )))(i)
}

fn response_code(i: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((tag_no_case("OK"), multispace1)))(i)
}

fn response(i: &[u8]) -> IResult<&[u8], (&[u8], usize)> {
    let (i, response) = cmd_parameters(i)?;
    let (i, response_code) = response_code(i)?;

    Ok((i, (response, response.len() + response_code.len())))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::helpers::SliceExt;
    use crate::queues::{ComQueue, ResQueue, UrcQueue};
    use crate::urc_matcher::{DefaultUrcMatcher, UrcMatcherResult};
    use heapless::spsc::Queue;

    const TEST_RX_BUF_LEN: usize = 256;

    #[test]
    fn cmd_test() {
        let r = command(b"+CCID").unwrap();
        assert_eq!(r, (&b""[..], &b"+CCID"[..]));

        let r = command(b"+USORD: 3,16,\"16 bytes of data\"\r\n").unwrap();
        assert_eq!(r, (&b": 3,16,\"16 bytes of data\"\r\n"[..], &b"+USORD"[..]));

        let r = command(b"&H").unwrap();
        assert_eq!(r, (&b""[..], &b"&H"[..]));

        let r = command(b"\\Q").unwrap();
        assert_eq!(r, (&b""[..], &b"\\Q"[..]));

        let r = command(b"S10").unwrap();
        assert_eq!(r, (&b""[..], &b"S10"[..]));

        let r = command(b"I").unwrap();
        assert_eq!(r, (&b""[..], &b"I"[..]));
    }

    #[test]
    fn echo_test() {
        let (r, e) = echo(b"AT\r\n").unwrap();
        assert_eq!(r, &b""[..]);
        assert_eq!(e.len(), 4);

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
    }

    #[test]
    fn cmd_parameters_test() {
        let (r, e) = cmd_parameters(b"+USORD: 3,16,123\r\nOK\r\n").unwrap();
        assert_eq!(r, &b"+USORD: 3,16,123"[..]);
        assert_eq!(e.len(), 18);

        let (r, e) = cmd_parameters(
            b"+UMGC:\r\nPath 0:\r\n\r\n10,9384\r\nPath 1:\r\n12,8192\r\nPath 2:\r\n6,8192\r\nOK\r\n",
        )
        .unwrap();
        assert_eq!(r, &b"+USORD: 3,16,123"[..]);
        assert_eq!(e.len(), 18);
    }

    // #[test]
    // fn no_response() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"AT\r\r\n\r\n").unwrap();
    //     let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!((res, bytes), (DigestResult::None, 7));

    //     buf.rotate_left(bytes);
    //     buf.truncate(buf.len() - bytes);

    //     buf.extend_from_slice(b"OK\r\n").unwrap();
    //     let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!((res, bytes), (DigestResult::Response(Ok(&[])), 4));
    //     buf.rotate_left(bytes);
    //     buf.truncate(buf.len() - bytes);
    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    // }

    // #[test]
    // fn response() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
    //     let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!((res, bytes), (DigestResult::None, 15));
    //     buf.rotate_left(bytes);
    //     buf.truncate(buf.len() - bytes);

    //     buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
    //         .unwrap();
    //     let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

    //     assert_eq!((res, bytes), (DigestResult::None, 0));

    //     buf.rotate_left(bytes);
    //     buf.truncate(buf.len() - bytes);

    //     {
    //         let expectation =
    //             Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
    //                 .unwrap();
    //         assert_eq!(buf, expectation);
    //     }

    //     buf.extend_from_slice(b"OK\r\n").unwrap();
    //     {
    //         let expectation = Vec::<_, TEST_RX_BUF_LEN>::from_slice(
    //             b"+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n",
    //         )
    //         .unwrap();
    //         assert_eq!(buf, expectation);
    //     }
    //     let (result, bytes) = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!(
    //         result,
    //         DigestResult::Response(Ok(b"+USORD: 3,16,\"16 bytes of data\""))
    //     );
    //     buf.rotate_left(bytes);
    //     buf.truncate(buf.len() - bytes);

    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    // }

    // #[test]
    // fn multi_line_response() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"AT+GMR\r\r\n").unwrap();
    //     let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!((res, bytes), (DigestResult::None, 9));
    //     buf.rotate_left(bytes);
    //     buf.truncate(buf.len() - bytes);

    //     buf.extend_from_slice(b"AaT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19\r\nOK\r\n").unwrap();
    //     let (res, bytes) = digester.digest(&mut buf, &mut urc_matcher);

    //     {
    //         let expectation = b"AaT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19";
    //         assert_eq!(res, DigestResult::Response(Ok(expectation)));
    //     }
    //     buf.rotate_left(bytes);
    //     buf.truncate(buf.len() - bytes);
    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    // }

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

    // #[test]
    // fn read_error() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());

    //     buf.extend_from_slice(b"OK\r\n").unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    // }

    // #[test]
    // fn error_response() {
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

    //     buf.extend_from_slice(b"ERROR\r\n").unwrap();
    //     let result = digester.digest(&mut buf, &mut urc_matcher);

    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    //     assert_eq!(
    //         result,
    //         DigestResult::Response(Err(InternalError::Error(
    //             Vec::from_slice(b"ERROR").unwrap()
    //         )))
    //     );
    // }

    // /// By breaking up non-AT-commands into chunks, it's possible that
    // /// they're mistaken for AT commands due to buffer clearing.
    // ///
    // /// Regression test for #27.
    // #[test]
    // fn chunkwise_digest() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"THIS FORM").unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );
    //     buf.extend_from_slice(b"AT SUCKS\r\n").unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );
    // }

    // /// By sending AT-commands byte-by-byte, it's possible that
    // /// the command is incorrectly ignored due to buffer clearing.
    // ///
    // /// Regression test for #27.
    // #[test]
    // fn bytewise_digest() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     for byte in b"AT\r\n" {
    //         buf.extend_from_slice(&[*byte]).unwrap();
    //         assert_eq!(
    //             digester.digest(&mut buf, &mut urc_matcher),
    //             DigestResult::None
    //         );
    //     }
    // }

    // /// If an invalid response ends with a line terminator, the incomplete flag
    // /// should be cleared.
    // #[test]
    // fn invalid_line_with_termination() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"some status msg\r\n").unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     buf.extend_from_slice(b"AT+GMR\r\r\n").unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );
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

    // // Regression test for #87
    // #[test]
    // fn cpin_parsing() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

    //     buf.extend_from_slice(b"AT+CPIN?\r\r\n+CPIN: READY\r\n\r\nOK\r\n")
    //         .unwrap();

    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );
    //     assert_eq!(
    //         buf,
    //         Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+CPIN: READY\r\n\r\nOK\r\n").unwrap()
    //     );

    //     let result = digester.digest(&mut buf, &mut urc_matcher);

    //     assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
    //     assert_eq!(
    //         result,
    //         DigestResult::Response(Ok(Vec::from_slice(b"+CPIN: READY").unwrap()))
    //     );
    // }

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

    // #[test]
    // fn multi_line_response_with_ok() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, 1024>::new();

    //     buf.extend_from_slice(b"AT+URDBLOCK=\"response.txt\",0,512\r\r\n")
    //         .unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     buf.extend_from_slice(b"+URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"\r\nOK").unwrap();
    //     let result = digester.digest(&mut buf, &mut urc_matcher);

    //     assert_eq!(buf, Vec::<_, 1024>::new());
    //     {
    //         let expectation = Vec::<_, 1024>::from_slice(b"+URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"").unwrap();
    //         assert_eq!(result, DigestResult::Response(Ok(expectation)));
    //     }
    // }

    // #[test]
    // #[ignore = "Until https://github.com/BlackbirdHQ/atat/issues/98 is resolved"]
    // fn multi_cmd_multi_line_response_with_ok() {
    //     let mut digester = NomDigester::default();
    //     let mut urc_matcher = DefaultUrcMatcher::default();
    //     let mut buf = Vec::<u8, 2048>::new();

    //     buf.extend_from_slice(b"AT+CPIN?\r\r\n+CPIN: READY\r\n\r\nOK\r\n")
    //         .unwrap();

    //     buf.extend_from_slice(b"AT+URDBLOCK=\"response.txt\",0,512\r\r\n")
    //         .unwrap();
    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     buf.extend_from_slice(b"+URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"\r\nOK").unwrap();
    //     let result = digester.digest(&mut buf, &mut urc_matcher);

    //     assert_eq!(
    //         result,
    //         DigestResult::Response(Ok(Vec::<_, 2048>::from_slice(b"+CPIN: READY").unwrap()))
    //     );

    //     assert_eq!(
    //         digester.digest(&mut buf, &mut urc_matcher),
    //         DigestResult::None
    //     );

    //     let result = digester.digest(&mut buf, &mut urc_matcher);
    //     assert_eq!(buf, Vec::<_, 2048>::new());

    //     {
    //         let expectation = Vec::<_, 2048>::from_slice(b"+URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"").unwrap();
    //         assert_eq!(result, DigestResult::Response(Ok(expectation)));
    //     }
    // }
}
