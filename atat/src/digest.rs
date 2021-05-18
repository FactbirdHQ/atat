use crate::{
    atat_log,
    helpers::{get_line, LossyStr, SliceExt},
    urc_matcher::{UrcMatcher, UrcMatcherResult},
    InternalError,
};
use heapless::Vec;

pub trait Digester {
    /// Command line termination character S3 (Default = b'\r' ASCII: \[013\])
    const LINE_TERM_CHAR: u8 = b'\r';

    /// Response formatting character S4 (Default = b'\n' ASCII: \[010\])
    const FORMAT_CHAR: u8 = b'\n';

    fn reset(&mut self);

    fn force_receive_state(&mut self);

    fn digest<const L: usize>(
        &mut self,
        buf: &mut Vec<u8, L>,
        urc_matcher: &mut impl UrcMatcher,
    ) -> DigestResult<L>;
}

#[derive(Debug, PartialEq)]
pub enum DigestResult<const L: usize> {
    Urc(Vec<u8, L>),
    Response(Result<Vec<u8, L>, InternalError>),
    None,
}

/// State of the `DefaultDigester`, used to distiguish URCs from solicited
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

/// A Digester that tries to implement the basic AT standard.
/// This digester should work for most usecases of ATAT.
///
/// Implements a request-response AT digester capable of working with or without AT echo enabled.
#[derive(Debug, Default)]
pub struct DefaultDigester {
    /// Current processing state.
    state: State,

    /// A flag that is set to `true` when the buffer is cleared
    /// with an incomplete response.
    buf_incomplete: bool,
}

impl Digester for DefaultDigester {
    fn reset(&mut self) {
        self.state = State::Idle;
        self.buf_incomplete = false;
    }

    fn force_receive_state(&mut self) {
        self.state = State::ReceivingResponse;
    }

    #[allow(clippy::cognitive_complexity)]
    fn digest<const L: usize>(
        &mut self,
        buf: &mut Vec<u8, L>,
        urc_matcher: &mut impl UrcMatcher,
    ) -> DigestResult<L> {
        // Trim leading whitespace
        if buf.starts_with(&[Self::LINE_TERM_CHAR]) || buf.starts_with(&[Self::FORMAT_CHAR]) {
            *buf = Vec::from_slice(buf.trim_start(&[
                b'\t',
                b' ',
                Self::FORMAT_CHAR,
                Self::LINE_TERM_CHAR,
            ]))
            .unwrap();
        }

        if !buf.is_empty() {
            atat_log!(trace, "Digest {:?} / {:?}", self.state, LossyStr(buf));
        }

        match self.state {
            State::Idle => {
                // Handle AT echo responses
                if !self.buf_incomplete && buf.get(0..2) == Some(b"AT") {
                    if get_line::<L, L>(
                        buf,
                        &[Self::LINE_TERM_CHAR],
                        Self::LINE_TERM_CHAR,
                        Self::FORMAT_CHAR,
                        false,
                        false,
                        false,
                    )
                    .is_some()
                    {
                        self.state = State::ReceivingResponse;
                        self.buf_incomplete = false;
                        atat_log!(trace, "Switching to state ReceivingResponse");
                    }

                // Handle URCs
                } else if !self.buf_incomplete && buf.get(0) == Some(&b'+') {
                    // Try to apply the custom URC matcher
                    let handled = match urc_matcher.process(buf) {
                        UrcMatcherResult::NotHandled => false,
                        UrcMatcherResult::Incomplete => true,
                        UrcMatcherResult::Complete(urc) => {
                            return DigestResult::Urc(urc);
                        }
                    };

                    // Always run some bare minimum URC handler
                    if !handled {
                        if let Some(line) = get_line(
                            buf,
                            &[Self::LINE_TERM_CHAR],
                            Self::LINE_TERM_CHAR,
                            Self::FORMAT_CHAR,
                            true,
                            false,
                            false,
                        ) {
                            self.buf_incomplete = false;
                            return DigestResult::Urc(line);
                        }
                    }
                // Text sent by the device that is not a valid response type (e.g. starting
                // with "AT" or "+") can be ignored. Clear the buffer, but only if we can
                // ensure that we don't accidentally break a valid response.
                } else if self.buf_incomplete || buf.len() > 2 {
                    atat_log!(
                        error,
                        "Clearing buffer with invalid response (incomplete: {}, buflen: {})",
                        self.buf_incomplete,
                        buf.len()
                    );
                    
                    self.buf_incomplete = buf.is_empty()
                        || (buf.len() > 0
                            && buf.get(buf.len() - 1) != Some(&Self::LINE_TERM_CHAR)
                            && buf.get(buf.len() - 1) != Some(&Self::FORMAT_CHAR));

                    let removed = get_line::<L, L>(
                        buf,
                        &[Self::LINE_TERM_CHAR],
                        Self::LINE_TERM_CHAR,
                        Self::FORMAT_CHAR,
                        false,
                        false,
                        false,
                    );

                    if let Some(r) = removed {
                        atat_log!(debug, "Cleared partial buffer, removed {:?}", LossyStr(&r));
                    } else {
                        buf.clear();
                        atat_log!(debug, "Cleared partial buffer, removed everything");
                    }

                    // If the buffer wasn't cleared completely, that means that
                    // a newline was found. In that case, the buffer cannot be
                    // in an incomplete state.
                    if !buf.is_empty() {
                        self.buf_incomplete = false;
                    }
                }
            }
            State::ReceivingResponse => {
                let resp = if let Some(mut line) = get_line::<L, L>(
                    buf,
                    b"OK",
                    Self::LINE_TERM_CHAR,
                    Self::FORMAT_CHAR,
                    true,
                    false,
                    false,
                ) {
                    Ok(get_line(
                        &mut line,
                        &[Self::LINE_TERM_CHAR],
                        Self::LINE_TERM_CHAR,
                        Self::FORMAT_CHAR,
                        true,
                        true,
                        false,
                    )
                    .unwrap_or_else(Vec::new))
                } else if let Some(mut line) = get_line::<L, L>(
                    buf,
                    b"ERROR",
                    Self::LINE_TERM_CHAR,
                    Self::FORMAT_CHAR,
                    true,
                    false,
                    false,
                ) {
                    Err(InternalError::Error(
                        get_line(
                            &mut line,
                            &[Self::LINE_TERM_CHAR],
                            Self::LINE_TERM_CHAR,
                            Self::FORMAT_CHAR,
                            true,
                            true,
                            true,
                        )
                        .unwrap_or_else(|| Vec::from_slice(&line).unwrap_or_default()),
                    ))
                } else if get_line::<L, L>(
                    buf,
                    b">",
                    Self::LINE_TERM_CHAR,
                    Self::FORMAT_CHAR,
                    false,
                    false,
                    false,
                )
                .is_some()
                    || get_line::<L, L>(
                        buf,
                        b"@",
                        Self::LINE_TERM_CHAR,
                        Self::FORMAT_CHAR,
                        false,
                        false,
                        false,
                    )
                    .is_some()
                {
                    Ok(Vec::new())
                } else {
                    return DigestResult::None;
                };

                atat_log!(trace, "Switching to state Idle");
                self.state = State::Idle;
                return DigestResult::Response(resp);
            }
        }
        DigestResult::None
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod test {
    use super::*;
    use crate::helpers::SliceExt;
    use crate::queues::{ComQueue, ResQueue, UrcQueue};
    use crate::urc_matcher::{DefaultUrcMatcher, UrcMatcherResult};
    use crate::{digest::State, urc_matcher};
    use heapless::spsc::Queue;

    const TEST_RX_BUF_LEN: usize = 256;

    #[test]
    fn no_response() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT\r\r\n\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );

        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"OK\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::Response(Ok(Vec::new()))
        );
        assert_eq!(digester.state, State::Idle);
    }

    #[test]
    fn response() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );

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
        let result = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
        assert_eq!(digester.state, State::Idle);
        {
            let expectation =
                Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+USORD: 3,16,\"16 bytes of data\"")
                    .unwrap();
            assert_eq!(result, DigestResult::Response(Ok(expectation)));
        }
    }

    #[test]
    fn multi_line_response() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT+GMR\r\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"AT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19\r\nOK\r\n").unwrap();
        let result = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
        assert_eq!(digester.state, State::Idle);
        {
            let expectation = Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"AT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19").unwrap();
            assert_eq!(result, DigestResult::Response(Ok(expectation)));
        }
    }

    #[test]
    fn urc() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);

        buf.extend_from_slice(b"+UUSORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        let result = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
        assert_eq!(digester.state, State::Idle);
        {
            let expectation =
                Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+UUSORD: 3,16,\"16 bytes of data\"")
                    .unwrap();
            assert_eq!(result, DigestResult::Urc(expectation));
        }
    }

    #[test]
    fn read_error() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());

        buf.extend_from_slice(b"OK\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );

        assert_eq!(digester.state, State::Idle);
    }

    #[test]
    fn error_response() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"ERROR\r\n").unwrap();
        let result = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(digester.state, State::Idle);
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
        assert_eq!(
            result,
            DigestResult::Response(Err(InternalError::Error(
                Vec::from_slice(b"ERROR").unwrap()
            )))
        );
    }

    /// By breaking up non-AT-commands into chunks, it's possible that
    /// they're mistaken for AT commands due to buffer clearing.
    ///
    /// Regression test for #27.
    #[test]
    fn chunkwise_digest() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);

        buf.extend_from_slice(b"THIS FORM").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT SUCKS\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::Idle);
    }

    /// By sending AT-commands byte-by-byte, it's possible that
    /// the command is incorrectly ignored due to buffer clearing.
    ///
    /// Regression test for #27.
    #[test]
    fn bytewise_digest() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);

        for byte in b"AT\r\n" {
            buf.extend_from_slice(&[*byte]).unwrap();
            assert_eq!(
                digester.digest(&mut buf, &mut urc_matcher),
                DigestResult::None
            );
        }
        assert_eq!(digester.state, State::ReceivingResponse);
    }

    /// If an invalid response ends with a line terminator, the incomplete flag
    /// should be cleared.
    #[test]
    fn invalid_line_with_termination() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);

        buf.extend_from_slice(b"some status msg\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::Idle);

        buf.extend_from_slice(b"AT+GMR\r\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);
    }

    /// If a valid response follows an invalid response, the buffer should not
    /// be cleared in between.
    #[test]
    fn mixed_response() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);

        buf.extend_from_slice(b"some status msg\r\nAT+GMR\r\r\n")
            .unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);
    }

    #[test]
    fn clear_buf_complete() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"hello\r\ngoodbye\r\n").unwrap();
        assert_eq!(
            buf,
            Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"hello\r\ngoodbye\r\n").unwrap()
        );

        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"").unwrap());
    }

    #[test]
    fn clear_buf_partial() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"hello\r\nthere\r\ngoodbye\r\n")
            .unwrap();
        assert_eq!(
            buf,
            Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"hello\r\nthere\r\ngoodbye\r\n").unwrap()
        );

        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );

        assert_eq!(
            buf,
            Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"there\r\ngoodbye\r\n").unwrap()
        );

        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );

        assert_eq!(
            buf,
            Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"goodbye\r\n").unwrap()
        );

        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"").unwrap());
    }

    #[test]
    fn clear_buf_partial_no_newlines() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        buf.extend_from_slice(b"no newlines anywhere").unwrap();

        assert_eq!(
            buf,
            Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"no newlines anywhere").unwrap()
        );

        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );

        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"").unwrap());
    }

    #[test]
    fn custom_urc_matcher() {
        struct MyUrcMatcher {}
        impl UrcMatcher for MyUrcMatcher {
            fn process<const L: usize>(&mut self, buf: &mut Vec<u8, L>) -> UrcMatcherResult<L> {
                if buf.len() >= 6 && buf.get(0..6) == Some(b"+match") {
                    let data = buf.clone();
                    buf.truncate(0);
                    UrcMatcherResult::Complete(data)
                } else if buf.len() >= 4 && buf.get(0..4) == Some(b"+mat") {
                    UrcMatcherResult::Incomplete
                } else {
                    UrcMatcherResult::NotHandled
                }
            }
        }

        let mut digester = DefaultDigester::default();
        let mut urc_matcher = MyUrcMatcher {};
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        // Initial state
        assert_eq!(digester.state, State::Idle);

        // Check an URC that is not handled by MyUrcMatcher (fall back to default behavior)
        // Note that this requires the trailing newlines to be present!
        buf.extend_from_slice(b"+default-behavior\r\n").unwrap();
        let result = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!(digester.state, State::Idle);
        assert_eq!(
            result,
            DigestResult::Urc(Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+default-behavior").unwrap())
        );

        // Check an URC that is generally handled by MyUrcMatcher but
        // considered incomplete (not enough data). This will not yet result in
        // an URC being dispatched.
        buf.extend_from_slice(b"+mat").unwrap();
        let result = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!(digester.state, State::Idle);
        assert_eq!(result, DigestResult::None);

        // Make it complete!
        buf.extend_from_slice(b"ch").unwrap(); // Still no newlines, but this will still be picked up.unwrap()!
        let result = digester.digest(&mut buf, &mut urc_matcher);
        assert_eq!(digester.state, State::Idle);
        assert_eq!(
            result,
            DigestResult::Urc(Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+match").unwrap())
        );
    }

    #[test]
    fn numeric_error_response() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"+CME ERROR: 123\r\n").unwrap();
        let result = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(digester.state, State::Idle);
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
        assert_eq!(
            result,
            DigestResult::Response(Err(InternalError::Error(
                Vec::from_slice(b"+CME ERROR: 123").unwrap()
            )))
        );
    }

    #[test]
    fn verbose_error_response() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"+CME ERROR: Operation not allowed\r\n")
            .unwrap();
        let result = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(digester.state, State::Idle);
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
        assert_eq!(
            result,
            DigestResult::Response(Err(InternalError::Error(
                Vec::from_slice(b"+CME ERROR: Operation not allowed").unwrap()
            )))
        );
    }

    #[test]
    fn truncate_verbose_error_response() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT+USORD=3,16\r\n").unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
            .unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        buf.extend_from_slice(b"+CME ERROR: Operation not allowed.. This is a very long error message, that will never fit in my buffer!\r\n").unwrap();
        let result = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(digester.state, State::Idle);
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
        assert_eq!(
            result,
            DigestResult::Response(Err(InternalError::Error(
                Vec::from_slice(
                    b"+CME ERROR: Operation not allowed.. This is a very long error message, that will neve"
                )
                .unwrap()
            )))
        );
    }

    #[test]
    fn data_ready_prompt() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT+USECMNG=0,0,\"Verisign\",1758\r>")
            .unwrap();
        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);

        let result = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(digester.state, State::Idle);
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
        assert_eq!(result, DigestResult::Response(Ok(heapless::Vec::new())));
    }

    // Regression test for #87
    #[test]
    fn cpin_parsing() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT+CPIN?\r\r\n+CPIN: READY\r\n\r\nOK\r\n")
            .unwrap();

        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);
        assert_eq!(
            buf,
            Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+CPIN: READY\r\n\r\nOK\r\n").unwrap()
        );

        let result = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(digester.state, State::Idle);
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
        assert_eq!(
            result,
            DigestResult::Response(Ok(Vec::from_slice(b"+CPIN: READY").unwrap()))
        );
    }

    // Regression test for #87
    #[test]
    fn cpin_error() {
        let mut digester = DefaultDigester::default();
        let mut urc_matcher = DefaultUrcMatcher::default();
        let mut buf = Vec::<u8, TEST_RX_BUF_LEN>::new();

        assert_eq!(digester.state, State::Idle);
        buf.extend_from_slice(b"AT+CPIN?\r\r\n+CME ERROR: 10\r\n")
            .unwrap();

        assert_eq!(
            digester.digest(&mut buf, &mut urc_matcher),
            DigestResult::None
        );
        assert_eq!(digester.state, State::ReceivingResponse);
        assert_eq!(
            buf,
            Vec::<_, TEST_RX_BUF_LEN>::from_slice(b"+CME ERROR: 10\r\n").unwrap()
        );

        let result = digester.digest(&mut buf, &mut urc_matcher);

        assert_eq!(digester.state, State::Idle);
        assert_eq!(buf, Vec::<_, TEST_RX_BUF_LEN>::new());
        assert_eq!(
            result,
            DigestResult::Response(Err(InternalError::Error(
                Vec::from_slice(b"+CME ERROR: 10").unwrap()
            )))
        );
    }
}
