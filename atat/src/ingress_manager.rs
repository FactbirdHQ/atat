use heapless::{
    consts,
    spsc::{Consumer, Producer},
    ArrayLength, String,
};

use crate::error::Error;
use crate::Command;
use crate::Config;

type ResProducer = Producer<'static, Result<String<consts::U256>, Error>, consts::U5, u8>;
type UrcProducer = Producer<'static, String<consts::U64>, consts::U10, u8>;
type ComConsumer = Consumer<'static, Command, consts::U3, u8>;

fn get_line<L: ArrayLength<u8>, I: ArrayLength<u8>>(
    buf: &mut String<I>,
    s: &str,
    line_term_char: u8,
    format_char: u8,
    trim_response: bool,
    reverse: bool,
) -> Option<String<L>> {
    let ind = if reverse {
        buf.rmatch_indices(s).next()
    } else {
        buf.match_indices(s).next()
    };
    match ind {
        Some((mut index, _)) => {
            index += s.len();
            while match buf.get(index..=index) {
                Some(c) => c.as_bytes()[0] == line_term_char || c.as_bytes()[0] == format_char,
                _ => false,
            } {
                index += 1;
            }

            let return_string = {
                let part = unsafe { buf.get_unchecked(0..index) };
                String::from(if trim_response { part.trim() } else { part })
            };
            *buf = String::from(unsafe { buf.get_unchecked(index..buf.len()) });
            Some(return_string)
        }
        None => None,
    }
}

/// State of the IngressManager, used to destiguish URC's from solicited
/// responses
#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Idle,
    ReceivingResponse,
}

pub struct IngressManager {
    buf: String<consts::U256>,
    res_p: ResProducer,
    urc_p: UrcProducer,
    com_c: ComConsumer,
    state: State,
    /// Command line termination character S3 (Default = '\r' ASCII: \[013\])
    line_term_char: u8,
    /// Response formatting character S4 (Default = '\n' ASCII: \[010\])
    format_char: u8,
    echo_enabled: bool,
}

impl IngressManager {
    pub fn new(
        res_p: ResProducer,
        urc_p: UrcProducer,
        com_c: ComConsumer,
        config: &Config,
    ) -> Self {
        Self {
            state: State::Idle,
            buf: String::new(),
            res_p,
            urc_p,
            com_c,
            line_term_char: config.line_term_char,
            format_char: config.format_char,
            echo_enabled: config.at_echo_enabled,
        }
    }

    /// Write data into the internal buffer
    /// raw bytes being the core type allows the ingress manager to
    /// be abstracted over the communication medium.
    pub fn write(&mut self, data: &[u8]) {
        for byte in data {
            match self.buf.push(*byte as char) {
                Ok(_) => {}
                Err(_) => self.notify_response(Err(Error::Overflow)),
            }
        }
    }

    fn notify_response(&mut self, resp: Result<String<consts::U256>, Error>) {
        if self.res_p.ready() {
            self.res_p.enqueue(resp).ok();
        } else {
            // FIXME: Handle queue not being ready
        }
    }

    fn notify_urc(&mut self, resp: String<consts::U64>) {
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
                    self.buf.clear()
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

    pub fn digest(&mut self) {
        self.handle_com();
        if self.buf.starts_with(self.line_term_char as char)
            || self.buf.starts_with(self.format_char as char)
        {
            // TODO: Custom trim_start, that trims based on line_term_char and format_char
            self.buf = String::from(self.buf.trim_start());
        }
        if self.buf.len() > 0 {
            log::trace!("{:?} - [{:?}]\r", self.state, self.buf);
        }
        match self.state {
            State::Idle => {
                if self.echo_enabled && self.buf.starts_with("AT") {
                    if let Some(_) = get_line::<consts::U128, _>(
                        &mut self.buf,
                        // FIXME: Use `self.line_term_char` here
                        "\r",
                        self.line_term_char,
                        self.format_char,
                        false,
                        false,
                    ) {
                        self.state = State::ReceivingResponse;
                    }
                } else if !self.echo_enabled {
                    unimplemented!("Disabling AT echo is currently unsupported");
                } else if self.buf.starts_with('+') {
                    if let Some(line) = get_line(
                        &mut self.buf,
                        // FIXME: Use `self.line_term_char` here
                        "\r",
                        self.line_term_char,
                        self.format_char,
                        false,
                        false,
                    ) {
                        self.notify_urc(line);
                    }
                } else {
                    self.buf.clear();
                }
            }
            State::ReceivingResponse => {
                let resp = if let Some(mut line) = get_line::<consts::U128, _>(
                    &mut self.buf,
                    "OK",
                    self.line_term_char,
                    self.format_char,
                    true,
                    false,
                ) {
                    Ok(get_line(
                        &mut line,
                        "\r",
                        self.line_term_char,
                        self.format_char,
                        true,
                        true,
                    )
                    .unwrap_or_else(|| String::new()))
                } else if get_line::<consts::U128, _>(
                    &mut self.buf,
                    "ERROR",
                    self.line_term_char,
                    self.format_char,
                    false,
                    false,
                )
                .is_some()
                {
                    Err(Error::InvalidResponse)
                } else {
                    return;
                };

                self.notify_response(resp);
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
    use atat::Mode;
    use heapless::{consts, spsc::Queue, String};

    macro_rules! setup {
        ($config:expr) => {{
            static mut REQ_Q: Queue<Result<String<consts::U256>, Error>, consts::U5, u8> =
                Queue(heapless::i::Queue::u8());
            let (p, c) = unsafe { REQ_Q.split() };
            static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
                Queue(heapless::i::Queue::u8());
            let (urc_p, _urc_c) = unsafe { URC_Q.split() };
            static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
            let (_com_p, com_c) = unsafe { COM_Q.split() };
            (IngressManager::new(p, urc_p, com_c, &$config), c)
        }};
    }

    #[test]
    fn no_response() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, mut c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write("AT\r\r\n\r\n".as_bytes());
        at_pars.digest();

        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write("OK\r\n".as_bytes());
        at_pars.digest();
        assert_eq!(at_pars.state, State::Idle);
        assert_eq!(c.dequeue().unwrap(), Ok(String::<consts::U256>::from("")));
    }

    #[test]
    fn response() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, mut c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write("AT+USORD=3,16\r\n".as_bytes());
        at_pars.digest();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write("+USORD: 3,16,\"16 bytes of data\"\r\n".as_bytes());
        at_pars.digest();

        assert_eq!(
            at_pars.buf,
            String::<consts::U256>::from("+USORD: 3,16,\"16 bytes of data\"\r\n")
        );

        at_pars.write("OK\r\n".as_bytes());
        assert_eq!(
            at_pars.buf,
            String::<consts::U256>::from("+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n")
        );
        at_pars.digest();
        assert_eq!(at_pars.buf, String::<consts::U256>::from(""));
        assert_eq!(at_pars.state, State::Idle);
        assert_eq!(
            c.dequeue().unwrap(),
            Ok(String::<consts::U256>::from(
                "+USORD: 3,16,\"16 bytes of data\""
            ))
        );
    }

    #[test]
    fn multi_line_response() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, mut c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write("AT+GMR\r\r\n".as_bytes());
        at_pars.digest();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write("AT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19\r\nOK\r\n".as_bytes());
        at_pars.digest();

        assert_eq!(at_pars.buf, String::<consts::U256>::from(""));
        assert_eq!(at_pars.state, State::Idle);
        assert_eq!(c.dequeue().unwrap(), Ok(String::<consts::U256>::from("AT version:1.1.0.0(May 11 2016 18:09:56)\r\nSDK version:1.5.4(baaeaebb)\r\ncompile time:May 20 2016 15:08:19")));
    }

    #[test]
    fn urc() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, _) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);

        at_pars.write("+UUSORD: 3,16,\"16 bytes of data\"\r\n".as_bytes());
        at_pars.digest();
        assert_eq!(at_pars.buf, String::<consts::U256>::from(""));
        assert_eq!(at_pars.state, State::Idle);
    }

    #[test]
    fn overflow() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, mut c) = setup!(conf);

        for _ in 0..266 {
            at_pars.write(b"s");
        }
        at_pars.digest();
        assert_eq!(c.dequeue().unwrap(), Err(Error::Overflow));
    }

    #[test]
    fn read_error() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, _) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);

        assert_eq!(at_pars.buf, String::<consts::U256>::from(""));
        at_pars.write("OK\r\n".as_bytes());
        at_pars.digest();

        assert_eq!(at_pars.state, State::Idle);
    }

    #[test]
    fn error_response() {
        let conf = Config::new(Mode::Timeout);
        let (mut at_pars, mut c) = setup!(conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write("AT+USORD=3,16\r\n".as_bytes());
        at_pars.digest();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write("+USORD: 3,16,\"16 bytes of data\"\r\n".as_bytes());
        at_pars.digest();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write("ERROR\r\n".as_bytes());
        at_pars.digest();

        assert_eq!(at_pars.state, State::Idle);
        assert_eq!(at_pars.buf, String::<consts::U256>::from(""));
        assert_eq!(c.dequeue().unwrap(), Err(Error::InvalidResponse));
    }
}
