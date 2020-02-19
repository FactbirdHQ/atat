use embedded_hal::{serial, timer::CountDown};

use heapless::{consts, spsc::Producer, ArrayLength, String};

use crate::buffer::Buffer;
use crate::error::{Error, Result};
use crate::Config;

type RespProducer = Producer<'static, Result<String<consts::U256>>, consts::U10, u8>;

#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Idle,
    ReceivingResponse,
}

pub struct ATParser<Rx, RxBufferLen>
where
    Rx: serial::Read<u8>,
    RxBufferLen: ArrayLength<u8>,
{
    rx: Rx,
    rx_buf: Buffer<RxBufferLen>,
    res_p: RespProducer,
    state: State,
    /// Command line termination character S3 (Default = '\r' [013])
    line_term_char: char,
    /// Response formatting character S4 (Default = '\n' [010])
    format_char: char,
    echo_enabled: bool,
}

impl<Rx, RxBufferLen> ATParser<Rx, RxBufferLen>
where
    Rx: serial::Read<u8>,
    RxBufferLen: ArrayLength<u8>,
{
    pub fn new<T: CountDown>(mut rx: Rx, queue: RespProducer, config: &Config<T>) -> Self {
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
            res_p: queue,
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

    pub fn handle_irq(&mut self)
    where
        <Rx as serial::Read<u8>>::Error: core::fmt::Debug,
    {
        match block!(self.rx.read()) {
            Ok(c) => {
                if self.rx_buf.push(c).is_err() {
                    // Notify error response, and reset rx_buf
                    self.notify_response(Err(Error::Overflow));
                    self.rx_buf.buffer.clear();
                } else {
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
                                // Trim
                                // self.rx_buf.buffer.trim();

                                // if self.rx_buf.buffer.len() > 0 {
                                //     let resp = self.rx_buf.take(ind.0);
                                //     self.notify_response(Ok(resp));
                                // }

                                self.rx_buf.buffer.clear();
                            }
                        }
                        State::ReceivingResponse => {
                            if c as char == self.format_char {
                                let (ind, err) = if let Some(ind) =
                                    self.rx_buf.buffer.rmatch_indices("OK\r\n").next()
                                {
                                    (ind, None)
                                } else if let Some(ind) =
                                    self.rx_buf.buffer.rmatch_indices("ERROR\r\n").next()
                                {
                                    (ind, Some(Error::InvalidResponse))
                                } else {
                                    return;
                                };

                                // FIXME: Handle mutable borrow warning (mutable_borrow_reservation_conflict)
                                let resp = self.rx_buf.take(ind.0);

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
            Err(_e) => {}
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
    use void::Void;

    struct CdMock {
        time: u32,
    }

    impl CountDown for CdMock {
        type Time = u32;
        fn start<T>(&mut self, count: T)
        where
            T: Into<Self::Time>,
        {
            self.time = count.into();
        }
        fn wait(&mut self) -> nb::Result<(), Void> {
            Ok(())
        }
    }

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
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, mut c) = unsafe { REQ_Q.split() };
        let rx_mock = RxMock::new(String::from("AT+USORD=3,16\r\nOK\r\n"));
        let conf = Config::new(Mode::Timeout(CdMock { time: 0 }));
        let mut at_pars: ATParser<RxMock, consts::U256> = ATParser::new(rx_mock, p, &conf);

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
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, mut c) = unsafe { REQ_Q.split() };
        let rx_mock = RxMock::new(String::from(
            "AT+USORD=3,16\r\n+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n",
        ));
        let conf = Config::new(Mode::Timeout(CdMock { time: 0 }));
        let mut at_pars: ATParser<RxMock, consts::U256> = ATParser::new(rx_mock, p, &conf);

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
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, _c) = unsafe { REQ_Q.split() };
        let rx_mock = RxMock::new(String::from("+UUSORD: 3,16,\"16 bytes of data\"\r\nOK\r\n"));
        let conf = Config::new(Mode::Timeout(CdMock { time: 0 }));
        let mut at_pars: ATParser<RxMock, consts::U256> = ATParser::new(rx_mock, p, &conf);

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
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, mut c) = unsafe { REQ_Q.split() };
        let mut t = String::<consts::U512>::new();

        for _ in 0..266 {
            t.push('s').ok();
        }

        let rx_mock = RxMock::new(t);
        let conf = Config::new(Mode::Timeout(CdMock { time: 0 }));
        let mut at_pars: ATParser<RxMock, consts::U256> = ATParser::new(rx_mock, p, &conf);

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
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, _c) = unsafe { REQ_Q.split() };
        let rx_mock = RxMock::new(String::from("OK\r\n"));
        let conf = Config::new(Mode::Timeout(CdMock { time: 0 }));
        let mut at_pars: ATParser<RxMock, consts::U256> = ATParser::new(rx_mock, p, &conf);

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
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, mut c) = unsafe { REQ_Q.split() };
        let rx_mock = RxMock::new(String::from(
            "AT+USORD=3,16\r\n+USORD: 3,16,\"16 bytes of data\"\r\nERROR\r\n",
        ));
        let conf = Config::new(Mode::Timeout(CdMock { time: 0 }));
        let mut at_pars: ATParser<RxMock, consts::U256> = ATParser::new(rx_mock, p, &conf);

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

        if let Some(result) = c.dequeue() {
            match result {
                Err(e) => assert_eq!(e, Error::InvalidResponse),
                Ok(resp) => {
                    panic!("Dequeue Ok: {:?}", resp);
                }
            };
        } else {
            panic!("Dequeue None.")
        }
    }
}
