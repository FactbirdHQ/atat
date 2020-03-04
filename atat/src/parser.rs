use embedded_hal::serial;
use heapless::{consts, spsc::Producer, String};

use crate::buffer::Buffer;
use crate::error::{Error, Result};
use crate::Config;

type ResProducer = Producer<'static, Result<String<consts::U256>>, consts::U5, u8>;
type UrcProducer = Producer<'static, String<consts::U64>, consts::U10, u8>;

#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Idle,
    ReceivingResponse,
}

pub struct ATParser<Rx>
where
    Rx: serial::Read<u8>,
{
    rx: Rx,
    rx_buf: Buffer<consts::U256>,
    res_p: ResProducer,
    urc_p: UrcProducer,
    state: State,
    /// Command line termination character S3 (Default = '\r' [013])
    pub line_term_char: char,
    /// Response formatting character S4 (Default = '\n' [010])
    format_char: char,
    echo_enabled: bool,
}

impl<Rx> ATParser<Rx>
where
    Rx: serial::Read<u8>,
{
    pub fn new(mut rx: Rx, res_p: ResProducer, urc_p: UrcProducer, config: &Config) -> Self {
        while rx.read().is_ok() {
            // Fix for unit tests!
            if let Ok(c) = rx.read() {
                if c == 0xFF {
                    break;
                }
            }
        }

        Self {
            rx,
            state: State::Idle,
            rx_buf: Buffer::new(),
            res_p,
            urc_p,
            line_term_char: config.line_term_char,
            format_char: config.format_char,
            echo_enabled: config.at_echo_enabled,
        }
    }

    fn notify_response(&mut self, resp: Result<String<consts::U256>>) {
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

    pub fn handle_irq(&mut self)
    where
        <Rx as serial::Read<u8>>::Error: core::fmt::Debug,
    {
        if let Ok(c) = block!(self.rx.read()) {
            if self.rx_buf.push(c).is_err() {
                // Notify error response, and reset rx_buf
                self.notify_response(Err(Error::Overflow));
                self.rx_buf.buffer.clear();
            } else {
                if self.rx_buf.buffer.starts_with(self.format_char)
                    || self.rx_buf.buffer.starts_with(self.line_term_char)
                {
                    self.rx_buf.remove_first();
                }
                match self.state {
                    State::Idle => {
                        if self.echo_enabled
                            && self.rx_buf.buffer.starts_with("AT")
                            && self.rx_buf.buffer.ends_with("\r\n")
                        {
                            self.state = State::ReceivingResponse;
                            self.rx_buf.buffer.clear();
                        } else if !self.echo_enabled {
                            unimplemented!("Disabling AT echo is currently unsupported");
                        } else if self.rx_buf.buffer.ends_with("\r\n") {
                            let resp = self.rx_buf.take(self.rx_buf.buffer.len());
                            self.notify_urc(resp);
                            self.rx_buf.buffer.clear();
                        }
                    }
                    State::ReceivingResponse => {
                        if c as char == self.format_char {
                            let (index, err) = if let Some(index) =
                                self.rx_buf.buffer.rmatch_indices("OK\r\n").next()
                            {
                                (index.0, None)
                            } else if let Some(index) =
                                self.rx_buf.buffer.rmatch_indices("ERROR\r\n").next()
                            {
                                #[cfg(not(feature = "error-message"))]
                                let err = Error::InvalidResponse;
                                #[cfg(feature = "error-message")]
                                let err =
                                    Error::InvalidResponseWithMessage(self.rx_buf.buffer.clone());

                                (index.0, Some(err))
                            } else {
                                return;
                            };

                            let resp = self.rx_buf.take(index);
                            self.notify_response(match err {
                                None => Ok(resp),
                                Some(e) => Err(e),
                            });

                            self.state = State::Idle;
                            self.rx_buf.buffer.clear();
                        }
                    }
                }
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
    use nb;

    struct RxMock {
        cnt: usize,
        s: String<consts::U512>,
        cleared: bool,
    }

    impl RxMock {
        fn new(s: String<consts::U512>) -> Self {
            RxMock {
                cnt: 0,
                s,
                cleared: false,
            }
        }
    }

    impl serial::Read<u8> for RxMock {
        type Error = ();

        fn read(&mut self) -> nb::Result<u8, Self::Error> {
            if self.cleared {
                if self.cnt >= self.s.len() {
                    Err(nb::Error::Other(()))
                } else {
                    let r = Ok(self.s.clone().into_bytes()[self.cnt]);
                    self.cnt += 1;
                    r
                }
            } else {
                self.cleared = true;
                println!("Cleared");
                Err(nb::Error::Other(()))
            }
        }
    }

    #[test]
    fn no_response() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, mut c) = unsafe { REQ_Q.split() };
        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };
        let rx_mock = RxMock::new(String::from("AT+USORD=3,16\r\nOK\r\n"));
        let conf = Config::new(Mode::Timeout);
        let mut at_pars: ATParser<RxMock> = ATParser::new(rx_mock, p, urc_p, &conf);

        assert_eq!(at_pars.state, State::Idle);
        for _ in 0.."AT+USORD=3,16\r\n".len() {
            at_pars.handle_irq();
        }
        assert_eq!(at_pars.state, State::ReceivingResponse);

        for _ in 0.."OK\r\n".len() {
            at_pars.handle_irq();
        }
        assert_eq!(at_pars.state, State::Idle);

        if let Some(result) = c.dequeue() {
            match result {
                Ok(resp) => {
                    assert_eq!(resp, String::<consts::U256>::from(""));
                }
                Err(e) => panic!("Dequeue Some error: {:?}", e),
            };
        } else {
            panic!("Dequeue None.")
        }
    }

    #[test]
    fn response() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, mut c) = unsafe { REQ_Q.split() };
        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };

        let rx_mock = RxMock::new(String::from(
            "AT+USORD=3,16\r\n+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n",
        ));
        let conf = Config::new(Mode::Timeout);
        let mut at_pars: ATParser<RxMock> = ATParser::new(rx_mock, p, urc_p, &conf);

        assert_eq!(at_pars.state, State::Idle);
        for _ in 0.."AT+USORD=3,16\r\n".len() {
            at_pars.handle_irq();
        }
        assert_eq!(at_pars.state, State::ReceivingResponse);

        for _ in 0.."+USORD: 3,16,\"16 bytes of data\"\r\n".len() {
            at_pars.handle_irq();
        }

        for _ in 0.."OK\r\n".len() {
            at_pars.handle_irq();
        }
        assert_eq!(at_pars.rx_buf.buffer, String::<consts::U256>::from(""));
        assert_eq!(at_pars.state, State::Idle);

        if let Some(result) = c.dequeue() {
            match result {
                Ok(resp) => {
                    assert_eq!(
                        resp,
                        String::<consts::U256>::from("+USORD: 3,16,\"16 bytes of data\"")
                    );
                }
                Err(e) => panic!("Dequeue Some error: {:?}", e),
            };
        } else {
            panic!("Dequeue None.")
        }
    }

    #[test]
    fn ucr() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, _c) = unsafe { REQ_Q.split() };
        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };

        let rx_mock = RxMock::new(String::from("+UUSORD: 3,16,\"16 bytes of data\"\r\nOK\r\n"));
        let conf = Config::new(Mode::Timeout);
        let mut at_pars: ATParser<RxMock> = ATParser::new(rx_mock, p, urc_p, &conf);

        assert_eq!(at_pars.state, State::Idle);

        for _ in 0.."+UUSORD: 3,16,\"16 bytes of data\"\r\n".len() {
            at_pars.handle_irq();
        }
        assert_eq!(at_pars.rx_buf.buffer, String::<consts::U256>::from(""));
        for _ in 0.."OK\r\n".len() {
            at_pars.handle_irq();
        }
        assert_eq!(at_pars.rx_buf.buffer, String::<consts::U256>::from(""));
        assert_eq!(at_pars.state, State::Idle);
    }

    #[test]
    fn overflow() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, mut c) = unsafe { REQ_Q.split() };
        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };

        let mut t = String::<consts::U512>::new();

        for _ in 0..266 {
            t.push('s').ok();
        }

        let rx_mock = RxMock::new(t);
        let conf = Config::new(Mode::Timeout);
        let mut at_pars: ATParser<RxMock> = ATParser::new(rx_mock, p, urc_p, &conf);

        for _ in 0..266 {
            at_pars.handle_irq();
        }

        if let Some(result) = c.dequeue() {
            match result {
                Err(e) => assert_eq!(e, Error::Overflow),
                Ok(resp) => {
                    panic!("Dequeue Ok: {:?}", resp);
                }
            };
        } else {
            panic!("Dequeue None.")
        }
    }

    #[test]
    fn read_error() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, _c) = unsafe { REQ_Q.split() };
        let rx_mock = RxMock::new(String::from("OK\r\n"));
        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };

        let conf = Config::new(Mode::Timeout);
        let mut at_pars: ATParser<RxMock> = ATParser::new(rx_mock, p, urc_p, &conf);

        assert_eq!(at_pars.state, State::Idle);

        assert_eq!(at_pars.rx_buf.buffer, String::<consts::U256>::from(""));
        for _ in 0.."OK\r\n".len() + 1 {
            at_pars.handle_irq();
        }

        at_pars.rx.s.push('s').ok();
        assert_eq!(at_pars.state, State::Idle);
    }

    #[test]
    fn error_response() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, mut c) = unsafe { REQ_Q.split() };

        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };

        let rx_mock = RxMock::new(String::from(
            "AT+USORD=3,16\r\n+USORD: 3,16,\"16 bytes of data\"\r\nERROR\r\n",
        ));
        let conf = Config::new(Mode::Timeout);
        let mut at_pars: ATParser<RxMock> = ATParser::new(rx_mock, p, urc_p, &conf);

        assert_eq!(at_pars.state, State::Idle);
        for _ in 0.."AT+USORD=3,16\r\n".len() {
            at_pars.handle_irq();
        }
        assert_eq!(at_pars.state, State::ReceivingResponse);

        for _ in 0.."+USORD: 3,16,\"16 bytes of data\"\r\n".len() {
            at_pars.handle_irq();
        }

        for _ in 0.."ERROR\r\n".len() {
            at_pars.handle_irq();
        }

        assert_eq!(at_pars.rx_buf.buffer, String::<consts::U256>::from(""));
        assert_eq!(at_pars.state, State::Idle);

        #[cfg(feature = "error-message")]
        let expectation = Error::InvalidResponseWithMessage(String::from(
            "+USORD: 3,16,\"16 bytes of data\"\r\nERROR\r\n",
        ));
        #[cfg(not(feature = "error-message"))]
        let expectation = Error::InvalidResponse;

        if let Some(result) = c.dequeue() {
            match result {
                Err(e) => assert_eq!(e, expectation),
                Ok(resp) => {
                    panic!("Dequeue Ok: {:?}", resp);
                }
            };
        } else {
            panic!("Dequeue None.")
        }
    }
}
