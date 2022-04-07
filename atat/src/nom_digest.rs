use core::marker::PhantomData;

use crate::InternalError;

#[derive(Debug, PartialEq)]
pub enum DigestResult<'a> {
    Urc(&'a [u8]),
    Response(Result<&'a [u8], InternalError<'a>>),
    Prompt(u8),
    None,
}

pub trait Digester {
    fn digest<'a>(&mut self, buf: &'a [u8]) -> (DigestResult<'a>, usize);
}

pub trait Parser {
    fn parse<'a>(buf: &'a [u8])
        -> Result<(&'a [u8], usize), nom::Err<nom::error::Error<&'a [u8]>>>;
}

/// A Digester that tries to implement the basic AT standard.
/// This digester should work for most usecases of ATAT.
///
/// Implements a request-response AT digester capable of working with or without AT echo enabled.
///
/// Buffer can contain ('...' meaning arbitrary data):
/// - '...AT\<CMD>\r\r\n\<RESPONSE>\r\n\<RESPONSE CODE>\r\n...'             (Echo enabled)
/// - '...AT\<CMD>\r\r\n\<CMD>: \<PARAMETERS>\r\n\<RESPONSE CODE>\r\n...'   (Echo enabled)
/// - '...AT\<CMD>\r\r\n\<RESPONSE CODE>\r\n...'                            (Echo enabled)
/// - '...\<CMD>:\<PARAMETERS>\r\n\<RESPONSE CODE>\r\n...'                  (Echo disabled)
/// - '...\<RESPONSE>\r\n\<RESPONSE CODE>\r\n...'                           (Echo disabled)
/// - '...\<URC>\r\n...'                                                    (Unsolicited response code)
/// - '...\<URC>:\<PARAMETERS>\r\n...'                                      (Unsolicited response code)
/// - '...\<PROMPT>'                                                        (Prompt for data)
///
/// Goal of the digester is to extract these into:
/// - DigestResult::Response(Result\<RESPONSE>)
/// - DigestResult::Urc(\<URC>)
/// - DigestResult::Prompt(\<CHAR>)
/// - DigestResult::None
///
/// Usually \<RESPONSE CODE> is one of \['OK', 'ERROR', 'CME ERROR: \<NUMBER/STRING>', 'CMS ERROR: \<NUMBER/STRING>'],
/// but can be others as well depending on manufacturer.
///
/// Usually \<PROMPT> can be one of \['>', '@'], and is command specific and only valid for few selected commands.
pub struct AtDigester<P: Parser> {
    _urc_parser: PhantomData<P>,
    custom_success: fn(&[u8]) -> Result<(&[u8], usize), nom::Err<nom::error::Error<&[u8]>>>,
    custom_error: fn(&[u8]) -> Result<(&[u8], usize), nom::Err<nom::error::Error<&[u8]>>>,
    custom_prompt: fn(&[u8]) -> Result<(&[u8], usize), nom::Err<nom::error::Error<&[u8]>>>,
}

impl<P: Parser> AtDigester<P> {
    pub fn new() -> Self {
        Self {
            _urc_parser: PhantomData,
            custom_success: |_| Err(nom::Err::Incomplete(nom::Needed::Unknown)),
            custom_error: |_| Err(nom::Err::Incomplete(nom::Needed::Unknown)),
            custom_prompt: |_| Err(nom::Err::Incomplete(nom::Needed::Unknown)),
        }
    }
}

impl<P: Parser> Digester for AtDigester<P> {
    fn digest<'a>(&mut self, input: &'a [u8]) -> (DigestResult<'a>, usize) {
        // 1. Optionally discard echo
        let (buf, echo_bytes) = match nom::combinator::opt(parser::echo)(input) {
            Ok((buf, echo)) => (buf, echo.unwrap_or_default().len()),
            Err(nom::Err::Incomplete(_)) => return (DigestResult::None, 0),
            Err(_) => panic!("NOM ERROR - opt(echo)"),
        };

        // 2. Match for URC's
        if let Ok((urc, len)) = P::parse(input) {
            return (DigestResult::Urc(urc), len);
        }

        // 3. Parse for success responses
        // Custom successful replies first, if any
        if let Ok((response, len)) = (self.custom_success)(buf) {
            return (DigestResult::Response(Ok(response)), len + echo_bytes);
        }

        // Generic success replies
        if let Ok((_, (result, len))) = parser::success_response(buf) {
            return (result, len + echo_bytes);
        }

        // Custom prompts for data replies first, if any
        if let Ok((_response, len)) = (self.custom_prompt)(buf) {
            return (
                // FIXME:
                DigestResult::Prompt(b'p'),
                len + echo_bytes,
            );
        }

        // Generic prompts for data
        if let Ok((_, (result, len))) = parser::prompt_response(buf) {
            return (result, len + echo_bytes);
        }

        // 4. Parse for error responses
        // Custom error matches first, if any
        if let Ok((response, len)) = (self.custom_error)(buf) {
            return (
                DigestResult::Response(Err(InternalError::Custom(response))),
                len + echo_bytes,
            );
        }

        // Generic error matches
        if let Ok((_, (result, len))) = parser::error_response(buf) {
            return (result, len + echo_bytes);
        }

        // No matches at all. Just eat the echo and do nothing else.
        (
            DigestResult::None,
            parser::adjust_ending(&input[..echo_bytes]),
        )
    }
}

pub mod parser {
    use crate::error::{CmeError, CmsError, ConnectionError};

    use super::*;

    use core::str::FromStr;

    use nom::{
        branch::alt,
        bytes::streaming::tag,
        character::complete,
        combinator::{eof, map, map_res, not, recognize},
        error::ParseError,
        sequence::tuple,
        IResult,
    };

    pub fn adjust_ending(buf: &[u8]) -> usize {
        let mut len = buf.len();

        // Ends with one or more '<CR><LF>'
        while buf[..len].ends_with(b"\r\n") {
            len -= 2;
        }

        len
    }

    /// Matches the equivalent of regex: "\r\n{token}(.*)\r\n"
    pub fn urc_helper<'a, T, Error: ParseError<&'a [u8]>>(
        token: T,
    ) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], (&'a [u8], usize), Error>
    where
        &'a [u8]: nom::Compare<T> + nom::FindSubstring<T>,
        T: nom::InputLength + Clone + nom::InputTake + nom::InputIter,
    {
        move |i| {
            let (i, (le, urc_tag)) = tuple((
                complete::line_ending,
                recognize(tuple((tag(token.clone()), take_until_including("\r\n")))),
            ))(i)?;

            Ok((
                i,
                (trim_ascii_whitespace(urc_tag), le.len() + urc_tag.len()),
            ))
        }
    }

    pub fn error_response(buf: &[u8]) -> IResult<&[u8], (DigestResult, usize)> {
        alt((
            // Matches the equivalent of regex: "\r\n\+CME ERROR:\s*(\d+)\r\n"
            map(numeric_error("\r\n+CME ERROR:"), |(error_code, len)| {
                (
                    DigestResult::Response(Err(InternalError::CmeError(CmeError::from(
                        error_code,
                    )))),
                    len,
                )
            }),
            // Matches the equivalent of regex: "\r\n\+CMS ERROR:\s*(\d+)\r\n"
            map(numeric_error("\r\n+CMS ERROR:"), |(error_code, len)| {
                (
                    DigestResult::Response(Err(InternalError::CmsError(CmsError::from(
                        error_code,
                    )))),
                    len,
                )
            }),
            // Matches the equivalent of regex: "\r\n\+CME ERROR:\s*([^\n\r]+)\r\n"
            map(string_error("\r\n+CME ERROR:"), |(error_msg, len)| {
                (
                    DigestResult::Response(Err(InternalError::CmeError(CmeError::from_msg(
                        error_msg,
                    )))),
                    len,
                )
            }),
            // Matches the equivalent of regex: "\r\n\+CMS ERROR:\s*([^\n\r]+)\r\n"
            map(string_error("\r\n+CMS ERROR:"), |(error_msg, len)| {
                (
                    DigestResult::Response(Err(InternalError::CmsError(CmsError::from_msg(
                        error_msg,
                    )))),
                    len,
                )
            }),
            // Matches the equivalent of regex: "\r\nMODEM ERROR:\s*(\d+)\r\n"
            map(numeric_error("\r\nMODEM ERROR:"), |(_error_code, len)| {
                (
                    DigestResult::Response(Err(InternalError::CmeError(CmeError::Unknown))),
                    len,
                )
            }),
            map(generic_error(), |len| {
                (DigestResult::Response(Err(InternalError::Error)), len)
            }),
            map(connection_error(), |(err, len)| {
                (
                    DigestResult::Response(Err(InternalError::ConnectionError(err))),
                    len,
                )
            }),
            // TODO: Samsung Z810 may reply "NA" to report a not-available error
        ))(buf)
    }

    pub fn prompt_response(buf: &[u8]) -> IResult<&[u8], (DigestResult, usize)> {
        for prompt in &[b'>', b'@'] {
            if let Ok((buf, ((prefix, p), ws, _))) = tuple((
                take_until_including::<_, _, nom::error::Error<_>>(&[*prompt][..]),
                complete::multispace0,
                eof,
            ))(buf)
            {
                return Ok((
                    buf,
                    (
                        DigestResult::Prompt(*prompt),
                        prefix.len() + p.len() + ws.len(),
                    ),
                ));
            }
        }
        Err(nom::Err::Error(nom::error::Error::new(
            buf,
            nom::error::ErrorKind::NoneOf,
        )))
    }

    pub fn success_response(buf: &[u8]) -> IResult<&[u8], (DigestResult, usize)> {
        let (i, ((data, tag), ws)) = alt((
            tuple((
                take_until_including("\r\nOK\r\n"),
                nom::combinator::success(&b""[..]),
            )),
            tuple((
                take_until_including("\r\nCONNECT"),
                recognize(take_until_including("\r\n")),
            )),
        ))(buf)?;

        Ok((
            i,
            (
                DigestResult::Response(Ok(trim_ascii_whitespace(data))),
                data.len() + tag.len() + ws.len(),
            ),
        ))
    }

    /// Matches a full AT echo. Eg `AT+USORD=3,16\r\n`
    pub fn echo(buf: &[u8]) -> IResult<&[u8], &[u8]> {
        if buf.len() < 2 {
            return Ok((buf, &[]));
        }

        recognize(nom::bytes::complete::take_until("\r\n"))(buf)
    }

    fn take_until_including<'a, T, Input, Error: ParseError<Input>>(
        tag: T,
    ) -> impl Fn(Input) -> IResult<Input, (Input, Input), Error>
    where
        Input: nom::Compare<T> + nom::FindSubstring<T> + nom::InputLength + nom::InputTake,
        T: nom::InputLength + Clone + nom::InputTake,
    {
        move |i| {
            let (i, d) = nom::bytes::complete::take_until(tag.clone())(i)?;
            let (i, t) = nom::bytes::streaming::tag_no_case(tag.clone())(i)?;
            Ok((i, (d, t)))
        }
    }

    /// Matches the equivalent of regex: "{token}\s*(\d+)\r\n"
    fn numeric_error<'a, T, Error: ParseError<&'a [u8]>>(
        token: T,
    ) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], (u16, usize), Error>
    where
        &'a [u8]: nom::Compare<T> + nom::FindSubstring<T>,
        T: nom::InputLength + Clone + nom::InputTake + nom::InputIter,
        nom::Err<Error>: From<nom::Err<nom::error::Error<&'a [u8]>>>,
    {
        move |i| {
            let (i, (prefix_data, (error_code, error_code_len), le)) = tuple((
                recognize(tuple((
                    take_until_including(token.clone()),
                    complete::multispace0,
                ))),
                map_res(complete::digit1, |digits| {
                    u16::from_str(core::str::from_utf8(digits).map_err(drop)?)
                        .map_err(drop)
                        .map(|i| (i, digits.len()))
                }),
                complete::line_ending,
            ))(i)?;

            Ok((
                i,
                (error_code, prefix_data.len() + error_code_len + le.len()),
            ))
        }
    }

    /// Matches the equivalent of regex: "{token}\s*([^\n\r]+)\r\n"
    fn string_error<'a, T, Error: ParseError<&'a [u8]>>(
        token: T,
    ) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], (&'a [u8], usize), Error>
    where
        &'a [u8]: nom::Compare<T> + nom::FindSubstring<T>,
        T: nom::InputLength + Clone + nom::InputTake + nom::InputIter,
    {
        move |i| {
            let (i, (prefix_data, _, error_msg)) = tuple((
                recognize(take_until_including(token.clone())),
                not(tag("\r")),
                recognize(take_until_including("\r\n")),
            ))(i)?;

            Ok((
                i,
                (
                    trim_ascii_whitespace(error_msg),
                    prefix_data.len() + error_msg.len(),
                ),
            ))
        }
    }

    /// Matches the equivalent of regex: "\r\n(ERROR)|(COMMAND NOT SUPPORT)\r\n"
    fn generic_error<'a, Error: ParseError<&'a [u8]>>(
    ) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], usize, Error> {
        move |i: &[u8]| {
            let (i, (data, tag)) = alt((
                take_until_including("\r\nERROR\r\n"),
                take_until_including("\r\nCOMMAND NOT SUPPORT\r\n"),
            ))(i)?;

            Ok((i, data.len() + tag.len()))
        }
    }

    /// Matches the equivalent of regex: "\r\n(NO CARRIER)|(BUSY)|(NO ANSWER)|(NO DIALTONE)\r\n"
    fn connection_error<'a, Error: ParseError<&'a [u8]>>(
    ) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], (ConnectionError, usize), Error> {
        move |i: &[u8]| {
            alt((
                map(
                    take_until_including("\r\nNO CARRIER\r\n"),
                    |(data, tag): (&[u8], &[u8])| {
                        (ConnectionError::NoCarrier, data.len() + tag.len())
                    },
                ),
                map(
                    take_until_including("\r\nBUSY\r\n"),
                    |(data, tag): (&[u8], &[u8])| (ConnectionError::Busy, data.len() + tag.len()),
                ),
                map(
                    take_until_including("\r\nNO ANSWER\r\n"),
                    |(data, tag): (&[u8], &[u8])| {
                        (ConnectionError::NoAnswer, data.len() + tag.len())
                    },
                ),
                map(
                    take_until_including("\r\nNO DIALTONE\r\n"),
                    |(data, tag): (&[u8], &[u8])| {
                        (ConnectionError::NoDialtone, data.len() + tag.len())
                    },
                ),
            ))(i)
        }
    }

    fn trim_ascii_whitespace(x: &[u8]) -> &[u8] {
        let from = match x.iter().position(|x| !x.is_ascii_whitespace()) {
            Some(i) => i,
            None => return &x[0..0],
        };
        let to = x.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
        &x[from..=to]
    }

    // fn print_dbg(i: &[u8]) -> IResult<&[u8], &[u8]> {
    //     print_dbg_ident("")(i)
    // }

    // fn print_dbg_ident(ident: &'static str) -> impl Fn(&[u8]) -> IResult<&[u8], &[u8]> {
    //     move |i| {
    //         // println!("{:?} = {:?}", ident, LossyStr(i));
    //         Ok((i, &[]))
    //     }
    // }
}
#[cfg(test)]
mod test {
    use super::parser::*;
    use super::*;
    use crate::{
        error::{CmeError, CmsError, ConnectionError},
        helpers::LossyStr,
    };

    const TEST_RX_BUF_LEN: usize = 256;

    enum UrcTestParser {}

    impl Parser for UrcTestParser {
        fn parse<'a>(
            buf: &'a [u8],
        ) -> Result<(&'a [u8], usize), nom::Err<nom::error::Error<&'a [u8]>>> {
            let (_, r) = nom::branch::alt((urc_helper("+UUSORD"), urc_helper("+CIEV")))(buf)?;

            Ok(r)
        }
    }

    #[test]
    fn mm_echo_removal() {
        let tests: Vec<(&[u8], &[u8])> = vec![
            (b"\r\n", b"\r\n"),
            (b"\r", b"\r"),
            (b"\n", b"\n"),
            (
                b"this is a string that ends just with <CR>\r",
                b"this is a string that ends just with <CR>\r",
            ),
            (
                b"this is a string that ends just with <CR>\n",
                b"this is a string that ends just with <CR>\n",
            ),
            (b"\r\nthis is valid", b"\r\nthis is valid"),
            (b"a\r\nthis is valid", b"\r\nthis is valid"),
            (b"a\r\n", b"\r\n"),
            (b"all this string is to be considered echo\r\n", b"\r\n"),
            (
                b"all this string is to be considered echo\r\nthis is valid",
                b"\r\nthis is valid",
            ),
            (
                b"echo echo\r\nthis is valid\r\nand so is this",
                b"\r\nthis is valid\r\nand so is this",
            ),
            (
                b"\r\nthis is valid\r\nand so is this",
                b"\r\nthis is valid\r\nand so is this",
            ),
            (
                b"\r\nthis is valid\r\nand so is this\r\n",
                b"\r\nthis is valid\r\nand so is this\r\n",
            ),
        ];

        for (response, expected) in tests {
            println!("Testing: {:?}", LossyStr(response));

            match nom::combinator::opt(parser::echo)(response) {
                Ok((buf, _)) => {
                    assert_eq!(buf, expected);
                }
                Err(nom::Err::Incomplete(_)) => {}
                Err(_) => panic!("NOM ERROR - opt(echo)"),
            }
        }
    }

    #[test]
    fn mm_error() {
        let tests: Vec<(&[u8], DigestResult, usize)> = vec![
            (b"\r\nUNKNOWN COMMAND\r\n", DigestResult::None, 0),
            (
                b"\r\nERROR\r\n",
                DigestResult::Response(Err(InternalError::Error)),
                9,
            ),
            (
                b"\r\nERROR\r\n\r\noooops\r\n",
                DigestResult::Response(Err(InternalError::Error)),
                9,
            ),
            (
                b"\r\n+CME ERROR: raspberry\r\n",
                DigestResult::Response(Err(InternalError::CmeError(CmeError::Unknown))),
                25,
            ),
            (
                b"\r\n+CME ERROR: 112\r\n",
                DigestResult::Response(Err(InternalError::CmeError(CmeError::AreaNotAllowed))),
                19,
            ),
            (
                b"\r\n+CME ERROR: \r\n",
                DigestResult::Response(Err(InternalError::CmeError(CmeError::Unknown))),
                16,
            ),
            (b"\r\n+CME ERROR:\r\n", DigestResult::None, 0),
            (
                b"\r\n+CMS ERROR: bananas\r\n",
                DigestResult::Response(Err(InternalError::CmsError(CmsError::Unknown))),
                23,
            ),
            (
                b"\r\n+CMS ERROR: 332\r\n",
                DigestResult::Response(Err(InternalError::CmsError(CmsError::NetworkTimeout))),
                19,
            ),
            (
                b"\r\n+CMS ERROR: \r\n",
                DigestResult::Response(Err(InternalError::CmsError(CmsError::Unknown))),
                16,
            ),
            (b"\r\n+CMS ERROR:\r\n", DigestResult::None, 0),
            (
                b"\r\nMODEM ERROR: 5\r\n",
                DigestResult::Response(Err(InternalError::CmeError(CmeError::Unknown))),
                18,
            ),
            (b"\r\nMODEM ERROR: apple\r\n", DigestResult::None, 0),
            (b"\r\nMODEM ERROR: \r\n", DigestResult::None, 0),
            (b"\r\nMODEM ERROR:\r\n", DigestResult::None, 0),
            (
                b"\r\nCOMMAND NOT SUPPORT\r\n",
                DigestResult::Response(Err(InternalError::Error)),
                23,
            ),
            (
                b"\r\nCOMMAND NOT SUPPORT\r\n\r\nSomething extra\r\n",
                DigestResult::Response(Err(InternalError::Error)),
                23,
            ),
            (
                b"\r\nNO CARRIER\r\n",
                DigestResult::Response(Err(InternalError::ConnectionError(
                    ConnectionError::NoCarrier,
                ))),
                14,
            ),
            (
                b"\r\nNO CARRIER\r\n\r\nSomething extra\r\n",
                DigestResult::Response(Err(InternalError::ConnectionError(
                    ConnectionError::NoCarrier,
                ))),
                14,
            ),
            (
                b"\r\nBUSY\r\n",
                DigestResult::Response(Err(InternalError::ConnectionError(ConnectionError::Busy))),
                8,
            ),
            (
                b"\r\nBUSY\r\n\r\nSomething extra\r\n",
                DigestResult::Response(Err(InternalError::ConnectionError(ConnectionError::Busy))),
                8,
            ),
            (
                b"\r\nNO ANSWER\r\n",
                DigestResult::Response(Err(InternalError::ConnectionError(
                    ConnectionError::NoAnswer,
                ))),
                13,
            ),
            (
                b"\r\nNO ANSWER\r\n\r\nSomething extra\r\n",
                DigestResult::Response(Err(InternalError::ConnectionError(
                    ConnectionError::NoAnswer,
                ))),
                13,
            ),
            (
                b"\r\nNO DIALTONE\r\n",
                DigestResult::Response(Err(InternalError::ConnectionError(
                    ConnectionError::NoDialtone,
                ))),
                15,
            ),
            (
                b"\r\nNO DIALTONE\r\n\r\nSomething extra\r\n",
                DigestResult::Response(Err(InternalError::ConnectionError(
                    ConnectionError::NoDialtone,
                ))),
                15,
            ),
        ];

        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        for (response, expected_result, swallowed_bytes) in tests {
            buf.clear();

            buf.extend_from_slice(response).unwrap();
            let (res, bytes) = digester.digest(&mut buf);
            assert_eq!((res, bytes), (expected_result, swallowed_bytes));

            buf.rotate_left(bytes);
            buf.truncate(buf.len() - bytes);
        }
    }

    #[test]
    fn mm_ok() {
        let tests: Vec<(&[u8], DigestResult, usize)> = vec![
            (b"\r\nOK\r\n", DigestResult::Response(Ok(b"")), 6),
            (b"\r\nOK\r\n\r\n+CMTI: \"ME\",1\r\n", DigestResult::Response(Ok(b"")), 6),
            (b"\r\nOK\r\n\r\n+CIEV: 7,1\r\n\r\n+CRING: VOICE\r\n\r\n+CLIP: \"+0123456789\",145,,,,0\r\n", DigestResult::Response(Ok(b"")), 6),
            (b"\r\n+CIEV: 7,1\r\n\r\n+CRING: VOICE\r\n\r\n+CLIP: \"+0123456789\",145,,,,0\r\n", DigestResult::Urc(b"+CIEV: 7,1"), 14),
            (b"\r\nUNKNOWN COMMAND\r\n", DigestResult::None, 0),
        ];

        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        for (response, expected_result, swallowed_bytes) in tests {
            buf.clear();

            buf.extend_from_slice(response).unwrap();
            let (res, bytes) = digester.digest(&mut buf);
            assert_eq!((res, bytes), (expected_result, swallowed_bytes));

            buf.rotate_left(bytes);
            buf.truncate(buf.len() - bytes);
        }
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
        assert_eq!(r, &b"\r\n\r\n"[..]);
        assert_eq!(e.len(), 3);

        let (r, e) = echo(b"AT+USORD=3,16\r\n").unwrap();
        assert_eq!(r, &b"\r\n"[..]);
        assert_eq!(e.len(), 13);

        let (r, e) = echo(b"AT+CMUX=?\r\n").unwrap();
        assert_eq!(r, &b"\r\n"[..]);
        assert_eq!(e.len(), 9);

        let (r, e) = echo(b"AT+CMUX?\r\n").unwrap();
        assert_eq!(r, &b"\r\n"[..]);
        assert_eq!(e.len(), 8);

        let (r, e) = echo(b"AT+CMUX?\r\nAT").unwrap();
        assert_eq!(r, &b"\r\nAT"[..]);
        assert_eq!(e.len(), 8);
    }

    #[test]
    fn response() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 13));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf);

        assert_eq!((res, bytes), (DigestResult::None, 0));

        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        {
            let expectation = b"\r\n+USORD: 3,16,\"16 bytes of data\"\r\n";
            assert_eq!(buf, expectation);
        }

        buf.extend_from_slice(b"OK\r\n").unwrap();
        {
            let expectation = b"\r\n+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n";
            assert_eq!(buf, expectation);
        }
        let (result, bytes) = digester.digest(&mut buf);
        assert_eq!(
            result,
            DigestResult::Response(Ok(b"+USORD: 3,16,\"16 bytes of data\""))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.is_empty());
    }

    #[test]
    fn urc_followed_by_command() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(
            b"\r\n+UUSORD: 0,5\r\nAT+USORD=0,4\r\r\n+USORD: 0,4,\"90030002\"\r\nOK\r\n",
        )
        .unwrap();
        let (res, bytes) = digester.digest(&mut buf);

        assert_eq!((res, bytes), (DigestResult::Urc(b"+UUSORD: 0,5"), 16));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert_eq!(
            &buf,
            b"AT+USORD=0,4\r\r\n+USORD: 0,4,\"90030002\"\r\nOK\r\n"
        );

        let (res, bytes) = digester.digest(&mut buf);

        assert_eq!(
            (res, bytes),
            (DigestResult::Response(Ok(b"+USORD: 0,4,\"90030002\"")), 43)
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert!(buf.is_empty());
    }

    #[test]
    fn response_no_echo() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"\r\n+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf);

        assert_eq!((res, bytes), (DigestResult::None, 0));

        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        {
            let expectation = b"\r\n+USORD: 3,16,\"16 bytes of data\"\r\n";
            assert_eq!(buf, expectation);
        }

        buf.extend_from_slice(b"OK\r\n").unwrap();
        {
            let expectation = b"\r\n+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n";
            assert_eq!(buf, expectation);
        }
        let (result, bytes) = digester.digest(&mut buf);
        assert_eq!(
            result,
            DigestResult::Response(Ok(b"+USORD: 3,16,\"16 bytes of data\""))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.is_empty());
    }

    #[test]
    fn multi_line_response() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+GMR\r\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 7));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"AT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19\r\nOK\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);

        let expectation = b"AT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19";
        assert_eq!(res, DigestResult::Response(Ok(expectation)));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert!(buf.is_empty());
    }

    #[test]
    fn urc_digest() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"\r\n+UUSORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!(
            (res, bytes),
            (DigestResult::Urc(b"+UUSORD: 3,16,\"16 bytes of data\""), 36)
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert!(buf.is_empty());
    }

    #[test]
    fn error_response() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 13));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"ERROR\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!(
            (res, bytes),
            (DigestResult::Response(Err(InternalError::Error)), 42)
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.is_empty());
    }

    /// By breaking up non-AT-commands into chunks, it's possible that
    /// they're mistaken for AT commands due to buffer clearing.
    ///
    /// Regression test for #27.
    #[test]
    fn garbage_cleanup() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"THIS FORM").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"AT SUCKS\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 17));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.starts_with(b"\r\n"));

        buf.extend_from_slice(b"@\n@").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();

        buf.extend_from_slice(b"+CME ERROR: 122\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);

        assert_eq!(
            res,
            DigestResult::Response(Err(InternalError::CmeError(CmeError::Congestion)))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.is_empty());

        buf.extend_from_slice(b"\r\n+UUSORD: 0,37\n+UUSORD: 0,371\r\n")
            .unwrap();

        let (res, bytes) = digester.digest(&mut buf);

        assert_eq!(res, DigestResult::Urc(b"+UUSORD: 0,37\n+UUSORD: 0,371"));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.is_empty());
    }

    /// By sending AT-commands byte-by-byte, it's possible that
    /// the command is incorrectly ignored due to buffer clearing.
    ///
    /// Regression test for #27.
    #[test]
    fn bytewise_digest() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"A").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"T").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"\r").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 0));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 2));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.starts_with(b"\r\n"));
    }

    #[test]
    fn numeric_error_response() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();

        buf.extend_from_slice(b"+CME ERROR: 122\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);

        assert_eq!(
            res,
            DigestResult::Response(Err(InternalError::CmeError(CmeError::Congestion)))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.is_empty());
    }

    #[test]
    fn verbose_error_response() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();

        buf.extend_from_slice(b"+CME ERROR: Operation not allowed\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf);

        assert_eq!(
            res,
            DigestResult::Response(Err(InternalError::CmeError(CmeError::NotAllowed)))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.is_empty());
    }

    #[test]
    fn data_ready_prompt() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USECMNG=0,0,\"Verisign\",1758\r>")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::Prompt(b'>'), 32));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
    }

    #[test]
    fn ready_for_data_prompt() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+USOWR=3,16\r@").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::Prompt(b'@'), 15));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
    }

    #[test]
    fn without_prefix() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        // With echo enabled
        buf.extend_from_slice(b"AT+CIMI?\r\n123456789\r\nOK\r\n")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::Response(Ok(b"123456789")), 25));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert!(buf.is_empty());

        // Without echo enabled
        buf.extend_from_slice(b"\r\n123456789\r\nOK\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::Response(Ok(b"123456789")), 17));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert!(buf.is_empty());
    }

    // Regression test for #87
    #[test]
    fn cpin_parsing() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+CPIN?\r\r\n+CPIN: READY\r\n\r\nOK\r\n")
            .unwrap();

        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!(
            (res, bytes),
            (DigestResult::Response(Ok(b"+CPIN: READY")), 31)
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert!(buf.is_empty());
    }

    // Regression test for #87
    #[test]
    fn cpin_error() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"AT+CPIN?\r\r\n+CME ERROR: 10\r\n")
            .unwrap();

        let (res, bytes) = digester.digest(&mut buf);

        assert_eq!(
            res,
            DigestResult::Response(Err(InternalError::CmeError(CmeError::SimNotInserted)))
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.is_empty());
    }

    #[test]
    fn multi_line_response_with_ok() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, 1024>::new();

        buf.extend_from_slice(b"AT+URDBLOCK=\"response.txt\",0,512\r\r\n+")
            .unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 33));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        buf.extend_from_slice(b"URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"\r\nOK\r\n").unwrap();
        let (res, bytes) = digester.digest(&mut buf);
        let expectation = b"+URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"";
        assert_eq!((res, bytes), (DigestResult::Response(Ok(expectation)), 552));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.is_empty());
    }

    #[test]
    fn multi_cmd_multi_line_response_with_ok() {
        let mut digester = AtDigester::<UrcTestParser>::new();
        let mut buf = heapless::Vec::<u8, 2048>::new();

        buf.extend_from_slice(b"AT+CPIN?\r\r\n+CPIN: READY\r\n\r\nOK\r\n")
            .unwrap();

        buf.extend_from_slice(b"AT+URDBLOCK=\"response.txt\",0,512\r\r\n+")
            .unwrap();

        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!(
            (res, bytes),
            (DigestResult::Response(Ok(b"+CPIN: READY")), 31)
        );
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert_eq!(buf, b"AT+URDBLOCK=\"response.txt\",0,512\r\r\n+");

        let (res, bytes) = digester.digest(&mut buf);
        assert_eq!((res, bytes), (DigestResult::None, 33));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);
        assert_eq!(buf, b"\r\n+");

        buf.extend_from_slice(b"URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"\r\nOK\r\n").unwrap();

        let (res, bytes) = digester.digest(&mut buf);
        let expectation = b"+URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"";
        assert_eq!((res, bytes), (DigestResult::Response(Ok(expectation)), 552));
        buf.rotate_left(bytes);
        buf.truncate(buf.len() - bytes);

        assert!(buf.is_empty());
    }
}
