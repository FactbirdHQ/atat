use heapless::{consts, ArrayLength, Vec};

use crate::error::Error;
use crate::queues::{ComConsumer, ResProducer, UrcProducer};
use crate::{Command, Config};

use core::iter::FromIterator;

trait SliceExt {
    fn trim(&self, whitespaces: &[u8]) -> &Self;
    fn trim_start(&self, whitespaces: &[u8]) -> &Self;
}

impl SliceExt for [u8] {
    fn trim(&self, whitespaces: &[u8]) -> &[u8] {
        let is_not_whitespace = |c| !whitespaces.contains(c);

        if let Some(first) = self.iter().position(is_not_whitespace) {
            if let Some(last) = self.iter().rposition(is_not_whitespace) {
                &self[first..=last]
            } else {
                unreachable!();
            }
        } else {
            &[]
        }
    }

    fn trim_start(&self, whitespaces: &[u8]) -> &[u8] {
        let is_not_whitespace = |c| !whitespaces.contains(c);

        if let Some(first) = self.iter().position(is_not_whitespace) {
            &self[first..]
        } else {
            &[]
        }
    }
}

/// Helper function to take a subsection from `buf`.
///
/// It searches for `needle`, either from the beginning of buf, or the end,
/// depending on `reverse`. If the search finds a match, it continues forward as
/// long as the next characters matches `line_term_char` or `format_char`. It
/// then returns a substring, trimming it for whitespaces if `trim_response` is
/// true, and leaves the remainder in `buf`.
///
/// Example:
/// ```
/// let mut buf = heapless::String::from("+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\nAT+GMR\r\r\n");
/// let response: heapless::String<heapless::consts::U64> = get_line(&mut buf, "OK", b'\r', b'\n', false, false);
/// assert_eq!(response, heapless::String::from("+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n"));
/// assert_eq!(buf, heapless::String::from("AT+GMR\r\r\n"));
/// ```
pub(crate) fn get_line<L: ArrayLength<u8>, I: ArrayLength<u8>>(
    buf: &mut Vec<u8, I>,
    needle: &[u8],
    line_term_char: u8,
    format_char: u8,
    trim_response: bool,
    reverse: bool,
) -> Option<Vec<u8, L>> {
    if buf.len() == 0 {
        return None;
    }

    let ind = if reverse {
        buf.windows(needle.len())
            .rposition(|window| window == needle)
    } else {
        buf.windows(needle.len())
            .position(|window| window == needle)
    };

    match ind {
        Some(index) => {
            let white_space = buf
                .iter()
                .skip(index + needle.len())
                .position(|c| ![format_char, line_term_char].contains(c))
                .unwrap_or(buf.len() - index - needle.len());

            let (left, right) = buf.split_at(index + needle.len() + white_space);

            let return_buf = Vec::from_iter(
                if trim_response {
                    left.trim(&[b'\t', b' ', format_char, line_term_char])
                } else {
                    left
                }
                .iter()
                .cloned(),
            );

            *buf = Vec::from_iter(right.iter().cloned());
            Some(return_buf)
        }
        None => None,
    }
}

/// State of the IngressManager, used to distiguish URCs from solicited
/// responses
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum State {
    Idle,
    ReceivingResponse,
}

/// The type returned from a custom URC matcher.
pub enum UrcMatcherResult<L: ArrayLength<u8>> {
    NotHandled,
    Incomplete,
    Complete(Vec<u8, L>),
}

/// A user-defined URC matcher
///
/// This is used to detect and consume URCs that are not terminated with
/// standard response codes like "OK". An example could be an URC that returns
/// length-value (LV) encoded data without a terminator.
///
/// Note that you should only detect and consume but not process URCs.
/// Processing should be done by an [`AtatUrc`](trait.AtatUrc.html)
/// implementation.
///
/// A very simplistic example that can only handle the URC `+FOO,xx` (with
/// `xx` being two arbitrary characters) followed by CRLF:
///
/// ```
/// use atat::{UrcMatcher, UrcMatcherResult};
/// use heapless::{consts, String};
///
/// struct FooUrcMatcher { }
///
/// impl UrcMatcher for FooUrcMatcher {
///     type MaxLen = consts::U9;
///
///     fn process(&mut self, buf: &mut String<consts::U256>) -> UrcMatcherResult<Self::MaxLen> {
///         if buf.starts_with("+FOO,") {
///             if buf.len() >= 9 {
///                 if &buf[7..9] == "\r\n" {
///                     // URC is complete
///                     let data = String::from(&buf[..9]);
///                     *buf = String::from(&buf[9..]);
///                     UrcMatcherResult::Complete(data)
///                 } else {
///                     // Invalid, reject
///                     UrcMatcherResult::NotHandled
///                 }
///             } else {
///                 // Insufficient data
///                 UrcMatcherResult::Incomplete
///             }
///         } else {
///             UrcMatcherResult::NotHandled
///         }
///     }
/// }
/// ```
pub trait UrcMatcher {
    /// The max length that an URC might have (e.g. `heapless::consts::U256`)
    type MaxLen: ArrayLength<u8>;

    /// Take a look at `buf`. Then:
    ///
    /// - If the buffer contains a full URC, remove these bytes from the buffer
    ///   and return [`Complete`] with the matched data.
    /// - If it contains an incomplete URC, return [`Incomplete`].
    /// - Otherwise, return [`NotHandled`].
    ///
    /// [`Complete`]: enum.UrcMatcherResult.html#variant.Complete
    /// [`Incomplete`]: enum.UrcMatcherResult.html#variant.Incomplete
    /// [`NotHandled`]: enum.UrcMatcherResult.html#variant.NotHandled
    fn process(&mut self, buf: &mut Vec<u8, consts::U256>) -> UrcMatcherResult<Self::MaxLen>;
}

/// A URC matcher that does nothing (it always returns [`NotHandled`][nothandled]).
///
/// [nothandled]: enum.UrcMatcherResult.html#variant.NotHandled
pub struct NoopUrcMatcher {}

impl UrcMatcher for NoopUrcMatcher {
    type MaxLen = consts::U256;
    fn process(&mut self, _: &mut Vec<u8, consts::U256>) -> UrcMatcherResult<Self::MaxLen> {
        UrcMatcherResult::NotHandled
    }
}

pub struct IngressManager<U: UrcMatcher = NoopUrcMatcher> {
    /// Buffer holding incoming bytes.
    buf: Vec<u8, consts::U256>,
    /// A flag that is set to `true` when the buffer is cleared
    /// with an incomplete response.
    buf_incomplete: bool,

    /// The response producer sends responses to the client
    res_p: ResProducer,
    /// The URC producer sends URCs to the client
    urc_p: UrcProducer,
    /// The command consumer receives commands from the client
    com_c: ComConsumer,

    /// Current processing state.
    state: State,
    /// Command line termination character S3 (Default = b'\r' ASCII: \[013\])
    line_term_char: u8,
    /// Response formatting character S4 (Default = b'\n' ASCII: \[010\])
    format_char: u8,
    echo_enabled: bool,

    /// Custom URC matcher.
    custom_urc_matcher: Option<U>,
}

impl IngressManager<NoopUrcMatcher> {
    pub fn new(res_p: ResProducer, urc_p: UrcProducer, com_c: ComConsumer, config: Config) -> Self {
        Self::with_custom_urc_matcher(res_p, urc_p, com_c, config, None)
    }
}

impl<U> IngressManager<U>
where
    U: UrcMatcher<MaxLen = consts::U256>,
{
    pub fn with_custom_urc_matcher(
        res_p: ResProducer,
        urc_p: UrcProducer,
        com_c: ComConsumer,
        config: Config,
        custom_urc_matcher: Option<U>,
    ) -> Self {
        Self {
            state: State::Idle,
            buf: Vec::new(),
            buf_incomplete: false,
            res_p,
            urc_p,
            com_c,
            line_term_char: config.line_term_char,
            format_char: config.format_char,
            echo_enabled: config.at_echo_enabled,
            custom_urc_matcher,
        }
    }

    /// Write data into the internal buffer raw bytes being the core type allows
    /// the ingress manager to be abstracted over the communication medium.
    ///
    /// This function should be called by the UART Rx, either in a receive
    /// interrupt, or a DMA interrupt, to move data from the peripheral into the
    /// ingress manager receive buffer.
    pub fn write(&mut self, data: &[u8]) {
        #[cfg(feature = "logging")]
        log::trace!("Receiving {} bytes", data.len());
        for byte in data {
            match self.buf.push(*byte) {
                Ok(_) => {}
                Err(_) => self.notify_response(Err(Error::Overflow)),
            }
        }
    }

    /// Notify the client that an appropriate response code, or error has been
    /// received
    fn notify_response(&mut self, resp: Result<Vec<u8, consts::U256>, Error>) {
        #[cfg(feature = "logging")]
        log::debug!("Received response: {:?}", &resp);
        if self.res_p.ready() {
            self.res_p.enqueue(resp).ok();
        } else {
            // FIXME: Handle queue not being ready
        }
    }

    /// Notify the client that an unsolicited response code (URC) has been
    /// received
    fn notify_urc(&mut self, resp: Vec<u8, consts::U256>) {
        #[cfg(feature = "logging")]
        log::debug!("Received URC: {:?}", &resp);
        if self.urc_p.ready() {
            self.urc_p.enqueue(resp).ok();
        } else {
            // FIXME: Handle queue not being ready
        }
    }

    /// Handle receiving internal config commands from the client.
    fn handle_com(&mut self) {
        if let Some(com) = self.com_c.dequeue() {
            match com {
                Command::ClearBuffer => {
                    self.state = State::Idle;
                    #[cfg(feature = "logging")]
                    log::debug!("Clearing buffer on timeout / {:?}", self.buf);
                    self.clear_buf(true);
                }
                Command::ForceState(state) => {
                    #[cfg(feature = "logging")]
                    log::trace!("Switching to state {:?}", state);
                    self.state = state;
                }
                Command::SetEcho(e) => {
                    self.echo_enabled = e;
                }
                Command::SetFormat(c) => {
                    self.format_char = c;
                }
                Command::SetLineTerm(c) => {
                    self.line_term_char = c;
                }
            }
        }
    }

    /// Clear the buffer.
    ///
    /// If `complete` is `true`, clear the entire buffer. Otherwise, only
    /// remove data until (and including) the first newline (or the entire
    /// buffer if no newline is present).
    fn clear_buf(&mut self, complete: bool) {
        if complete {
            self.buf.clear();
            #[cfg(feature = "logging")]
            log::trace!("Cleared complete buffer");
        } else {
            let removed = get_line::<consts::U128, _>(
                &mut self.buf,
                &[self.line_term_char],
                self.line_term_char,
                self.format_char,
                false,
                false,
            );
            match removed {
                #[allow(unused)]
                Some(r) => {
                    #[cfg(feature = "logging")]
                    log::trace!("Cleared partial buffer, removed {:?}", r);
                }
                None => {
                    self.buf.clear();
                    #[cfg(feature = "logging")]
                    log::trace!("Cleared partial buffer, removed everything");
                }
            }
        }
    }

    /// Process the receive buffer, checking for AT responses, URC's or errors
    ///
    /// This function should be called regularly for the ingress manager to work
    pub fn digest(&mut self) {
        // Handle commands
        self.handle_com();

        // Trim leading whitespace
        if let Some(c) = self.buf.get(0) {
            if c == &self.line_term_char || c == &self.format_char {
                let mut new_buf = Vec::new();
                new_buf
                    .extend_from_slice(self.buf.trim_start(&[
                        b'\t',
                        b' ',
                        self.format_char,
                        self.line_term_char,
                    ]))
                    .unwrap();
                self.buf = new_buf;
            }
        }

        #[cfg(feature = "logging")]
        log::trace!("Digest / {:?} / {:?}", self.state, self.buf);

        match self.state {
            State::Idle => {
                // The minimal buffer length that is resuired to identify all
                // types of responses (e.g. `AT` and `+`).
                let min_length = 2;

                // Echo is currently resuired
                if !self.echo_enabled {
                    unimplemented!("Disabling AT echo is currently unsupported");
                }

                // Handle AT echo responses
                if !self.buf_incomplete && self.buf.get(0..2) == Some(b"AT") {
                    if get_line::<consts::U256, _>(
                        &mut self.buf,
                        &[self.line_term_char],
                        self.line_term_char,
                        self.format_char,
                        false,
                        false,
                    )
                    .is_some()
                    {
                        self.state = State::ReceivingResponse;
                        self.buf_incomplete = false;
                        #[cfg(feature = "logging")]
                        log::trace!("Switching to state ReceivingResponse");
                    }

                // Handle URCs
                } else if !self.buf_incomplete && self.buf.get(0) == Some(&b'+') {
                    // Try to apply the custom URC matcher
                    let handled = if let Some(ref mut matcher) = self.custom_urc_matcher {
                        match matcher.process(&mut self.buf) {
                            UrcMatcherResult::NotHandled => false,
                            UrcMatcherResult::Incomplete => true,
                            UrcMatcherResult::Complete(urc) => {
                                self.notify_urc(urc);
                                true
                            }
                        }
                    } else {
                        false
                    };
                    if !handled {
                        if let Some(line) = get_line(
                            &mut self.buf,
                            &[self.line_term_char],
                            self.line_term_char,
                            self.format_char,
                            false,
                            false,
                        ) {
                            self.buf_incomplete = false;
                            self.notify_urc(line);
                        }
                    }

                // Text sent by the device that is not a valid response type (e.g. starting
                // with "AT" or "+") can be ignored. Clear the buffer, but only if we can
                // ensure that we don't accidentally break a valid response.
                } else if self.buf_incomplete || self.buf.len() > min_length {
                    #[cfg(feature = "logging")]
                    log::trace!(
                        "Clearing buffer with invalid response (incomplete: {}, buflen: {})",
                        self.buf_incomplete,
                        self.buf.len(),
                    );
                    self.buf_incomplete = self.buf.len() > 0
                        && self.buf.get(self.buf.len() - 1) != Some(&self.line_term_char)
                        && self.buf.get(self.buf.len() - 1) != Some(&self.format_char);
                    self.clear_buf(false);

                    // If the buffer wasn't cleared completely, that means that
                    // a newline was found. In that case, the buffer cannot be
                    // in an incomplete state.
                    if !self.buf.is_empty() {
                        self.buf_incomplete = false;
                    }
                }
            }
            State::ReceivingResponse => {
                let resp = if let Some(mut line) = get_line::<consts::U256, _>(
                    &mut self.buf,
                    b"OK",
                    self.line_term_char,
                    self.format_char,
                    true,
                    false,
                ) {
                    Ok(get_line(
                        &mut line,
                        &[self.line_term_char],
                        self.line_term_char,
                        self.format_char,
                        true,
                        true,
                    )
                    .unwrap_or_else(Vec::new))
                } else if get_line::<consts::U256, _>(
                    &mut self.buf,
                    b"ERROR",
                    self.line_term_char,
                    self.format_char,
                    false,
                    false,
                )
                .is_some()
                {
                    Err(Error::InvalidResponse)
                } else if get_line::<consts::U256, _>(
                    &mut self.buf,
                    b">",
                    self.line_term_char,
                    self.format_char,
                    false,
                    false,
                )
                .is_some()
                    || get_line::<consts::U256, _>(
                        &mut self.buf,
                        b"@",
                        self.line_term_char,
                        self.format_char,
                        false,
                        false,
                    )
                    .is_some()
                {
                    Ok(Vec::new())
                } else {
                    return;
                };

                self.notify_response(resp);
                #[cfg(feature = "logging")]
                log::trace!("Switching to state Idle");
                self.state = State::Idle;
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod test {
    use super::*;
    use crate as atat;
    use crate::queues::{ComQueue, ResQueue, UrcQueue};
    use atat::Mode;
    use heapless::{consts, spsc::Queue};

    macro_rules! setup {
        ($config:expr, $urch:expr) => {{
            static mut RES_Q: ResQueue = Queue(heapless::i::Queue::u8());
            let (res_p, res_c) = unsafe { RES_Q.split() };
            static mut URC_Q: UrcQueue = Queue(heapless::i::Queue::u8());
            let (urc_p, urc_c) = unsafe { URC_Q.split() };
            static mut COM_Q: ComQueue = Queue(heapless::i::Queue::u8());
            let (_com_p, com_c) = unsafe { COM_Q.split() };
            (
                IngressManager::with_custom_urc_matcher(res_p, urc_p, com_c, $config, $urch),
                res_c,
                urc_c,
            )
        }};
        ($config:expr) => {{
            let val: (IngressManager<NoopUrcMatcher>, _, _) = setup!($config, None);
            val
        }};
    }

    #[test]
    fn no_response() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, mut res_c, _urc_c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write(b"AT\r\r\n\r\n");
        at_pars.digest();

        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write(b"OK\r\n");
        at_pars.digest();
        assert_eq!(at_pars.state, State::Idle);
        assert_eq!(res_c.dequeue().unwrap(), Ok(Vec::<u8, consts::U256>::new()));
    }

    #[test]
    fn response() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, mut res_c, _urc_c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write(b"AT+USORD=3,16\r\n");
        at_pars.digest();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write(b"+USORD: 3,16,\"16 bytes of data\"\r\n");
        at_pars.digest();

        {
            let mut expectation = Vec::<u8, consts::U256>::new();
            expectation
                .extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\n")
                .unwrap();
            assert_eq!(at_pars.buf, expectation);
        }

        at_pars.write(b"OK\r\n");
        {
            let mut expectation = Vec::<u8, consts::U256>::new();
            expectation
                .extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n")
                .unwrap();
            assert_eq!(at_pars.buf, expectation);
        }
        at_pars.digest();
        assert_eq!(at_pars.buf, Vec::<u8, consts::U256>::new());
        assert_eq!(at_pars.state, State::Idle);
        {
            let mut expectation = Vec::<u8, consts::U256>::new();
            expectation
                .extend_from_slice(b"+USORD: 3,16,\"16 bytes of data\"")
                .unwrap();
            assert_eq!(res_c.dequeue().unwrap(), Ok(expectation));
        }
    }

    #[test]
    fn multi_line_response() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, mut res_c, _urc_c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write(b"AT+GMR\r\r\n");
        at_pars.digest();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write(b"AT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19\r\nOK\r\n");
        at_pars.digest();

        assert_eq!(at_pars.buf, Vec::<u8, consts::U256>::new());
        assert_eq!(at_pars.state, State::Idle);
        {
            let mut expectation = Vec::<u8, consts::U256>::new();
            expectation.extend_from_slice(b"AT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19").unwrap();
            assert_eq!(res_c.dequeue().unwrap(), Ok(expectation));
        }
    }

    #[test]
    fn urc() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, _res_c, _urc_c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);

        at_pars.write(b"+UUSORD: 3,16,\"16 bytes of data\"\r\n");
        at_pars.digest();
        assert_eq!(at_pars.buf, Vec::<u8, consts::U256>::new());
        assert_eq!(at_pars.state, State::Idle);
    }

    #[test]
    fn overflow() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, mut res_c, _urc_c) = setup!(conf);

        for _ in 0..266 {
            at_pars.write(b"s");
        }
        at_pars.digest();
        assert_eq!(res_c.dequeue().unwrap(), Err(Error::Overflow));
    }

    #[test]
    fn read_error() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, _res_c, _urc_c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);

        assert_eq!(at_pars.buf, Vec::<u8, consts::U256>::new());
        at_pars.write(b"OK\r\n");
        at_pars.digest();

        assert_eq!(at_pars.state, State::Idle);
    }

    #[test]
    fn error_response() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, mut res_c, _urc_c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write(b"AT+USORD=3,16\r\n");
        at_pars.digest();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write(b"+USORD: 3,16,\"16 bytes of data\"\r\n");
        at_pars.digest();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write(b"ERROR\r\n");
        at_pars.digest();

        assert_eq!(at_pars.state, State::Idle);
        assert_eq!(at_pars.buf, Vec::<u8, consts::U256>::new());
        assert_eq!(res_c.dequeue().unwrap(), Err(Error::InvalidResponse));
    }

    /// By breaking up non-AT-commands into chunks, it's possible that
    /// they're mistaken for AT commands due to buffer clearing.
    ///
    /// Regression test for #27.
    #[test]
    fn chunkwise_digest() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, _res_c, _urc_c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);

        at_pars.write(b"THIS FORM");
        at_pars.digest();
        assert_eq!(at_pars.state, State::Idle);
        at_pars.write(b"AT SUCKS\r\n");
        at_pars.digest();
        assert_eq!(at_pars.state, State::Idle);
    }

    /// By sending AT-commands byte-by-byte, it's possible that
    /// the command is incorrectly ignored due to buffer clearing.
    ///
    /// Regression test for #27.
    #[test]
    fn bytewise_digest() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, _res_c, _urc_c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);

        for byte in b"AT\r\n" {
            at_pars.write(&[*byte]);
            at_pars.digest();
        }
        assert_eq!(at_pars.state, State::ReceivingResponse);
    }

    /// If an invalid response ends with a line terminator, the incomplete flag
    /// should be cleared.
    #[test]
    fn invalid_line_with_termination() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, _res_c, _urc_c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);

        at_pars.write(b"some status msg\r\n");
        at_pars.digest();
        assert_eq!(at_pars.state, State::Idle);

        at_pars.write(b"AT+GMR\r\r\n");
        at_pars.digest();
        assert_eq!(at_pars.state, State::ReceivingResponse);
    }

    /// If a valid response follows an invalid response, the buffer should not
    /// be cleared in between.
    #[test]
    fn mixed_response() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, _res_c, _urc_c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);

        at_pars.write(b"some status msg\r\nAT+GMR\r\r\n");
        at_pars.digest();
        at_pars.digest();
        assert_eq!(at_pars.state, State::ReceivingResponse);
    }

    #[test]
    fn clear_buf_complete() {
        let conf = Config::new(Mode::Timeout);
        let (mut ingress, _res_c, _urc_c) = setup!(conf);

        ingress.write(b"hello\r\ngoodbye\r\n");
        assert_eq!(ingress.buf, b"hello\r\ngoodbye\r\n");

        ingress.clear_buf(true);
        assert_eq!(ingress.buf, b"");
    }

    #[test]
    fn clear_buf_partial() {
        let conf = Config::new(Mode::Timeout);
        let (mut ingress, _res_c, _urc_c) = setup!(conf);

        ingress.write(b"hello\r\nthere\r\ngoodbye\r\n");
        assert_eq!(ingress.buf, b"hello\r\nthere\r\ngoodbye\r\n");

        ingress.clear_buf(false);
        assert_eq!(ingress.buf, b"there\r\ngoodbye\r\n");

        ingress.clear_buf(false);
        assert_eq!(ingress.buf, b"goodbye\r\n");

        ingress.clear_buf(false);
        assert_eq!(ingress.buf, b"");
    }

    #[test]
    fn clear_buf_partial_no_newlines() {
        let conf = Config::new(Mode::Timeout);
        let (mut ingress, _res_c, _urc_c) = setup!(conf);

        ingress.write(b"no newlines anywhere");
        assert_eq!(ingress.buf, b"no newlines anywhere");
        ingress.clear_buf(false);
        assert_eq!(ingress.buf, b"");
    }

    #[test]
    fn custom_urc_matcher() {
        let conf = Config::new(Mode::Timeout);

        struct MyUrcMatcher {}
        impl UrcMatcher for MyUrcMatcher {
            type MaxLen = consts::U256;
            fn process(
                &mut self,
                buf: &mut Vec<u8, consts::U256>,
            ) -> UrcMatcherResult<Self::MaxLen> {
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

        let (mut ingress, _res_c, mut urc_c) = setup!(conf, Some(MyUrcMatcher {}));

        // Initial state
        assert_eq!(ingress.state, State::Idle);
        assert_eq!(urc_c.dequeue(), None);

        // Check an URC that is not handled by MyUrcMatcher (fall back to default behavior)
        // Note that this resuires the trailing newlines to be present!
        ingress.write(b"+default-behavior\r\n");
        ingress.digest();
        assert_eq!(ingress.state, State::Idle);
        assert_eq!(urc_c.dequeue().unwrap(), b"+default-behavior\r\n");

        // Check an URC that is generally handled by MyUrcMatcher but
        // considered incomplete (not enough data). This will not yet result in
        // an URC being dispatched.
        ingress.write(b"+mat");
        ingress.digest();
        assert_eq!(ingress.state, State::Idle);
        assert_eq!(urc_c.dequeue(), None);

        // Make it complete!
        ingress.write(b"ch"); // Still no newlines, but this will still be picked up!
        ingress.digest();
        assert_eq!(ingress.state, State::Idle);
        assert_eq!(urc_c.dequeue().unwrap(), b"+match");
    }
}
