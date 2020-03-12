use heapless::{
    consts,
    spsc::{Consumer, Producer},
    ArrayLength, String,
};

use crate::error::{Error, Result};
use crate::Command;
use crate::Config;

type ResProducer = Producer<'static, Result<String<consts::U256>>, consts::U5, u8>;
type UrcProducer = Producer<'static, String<consts::U64>, consts::U10, u8>;
type ComConsumer = Consumer<'static, Command, consts::U3, u8>;

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
    /// Command line termination character S3 (Default = '\r' [013])
    line_term_char: char,
    /// Response formatting character S4 (Default = '\n' [010])
    format_char: char,
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

    fn take_trim_substring<L: ArrayLength<u8>>(&mut self, index: usize) -> String<L> {
        let mut result = String::new();
        let mut return_string: String<L> = String::new();
        return_string
            .push_str(unsafe { self.buf.get_unchecked(0..index).trim() })
            .ok();
        result
            .push_str(unsafe { self.buf.get_unchecked(index..self.buf.len()) })
            .ok();
        self.buf = result;
        return_string
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

    pub fn parse_at(&mut self) {
        self.handle_com();
        match self.state {
            State::Idle => {
                if self.echo_enabled && self.buf.starts_with("AT") {
                    if let Some((index, _)) = self.buf.match_indices("\r\n").next() {
                        self.state = State::ReceivingResponse;
                        self.take_trim_substring::<consts::U64>(index + 2);
                    }
                } else if !self.echo_enabled {
                    unimplemented!("Disabling AT echo is currently unsupported");
                } else if self.buf.starts_with('+') {
                    let resp = self.take_trim_substring(self.buf.len());
                    self.notify_urc(resp);
                    self.buf.clear();
                } else {
                    self.buf.clear();
                }
            }
            State::ReceivingResponse => {
                // TODO: Use `self.format_char` and `self.line_term_char` in these rmatches
                let (index, err) = if let Some(index) = self.buf.rmatch_indices("OK\r\n").next() {
                    (index.0, None)
                } else if let Some(index) = self.buf.rmatch_indices("ERROR\r\n").next() {
                    #[cfg(not(feature = "error-message"))]
                    let err = Error::InvalidResponse;
                    #[cfg(feature = "error-message")]
                    let err = Error::InvalidResponseWithMessage(self.buf.clone());

                    (index.0, Some(err))
                } else {
                    return;
                };

                let resp = self.take_trim_substring(index);
                self.notify_response(match err {
                    None => Ok(resp),
                    Some(e) => Err(e),
                });

                self.state = State::Idle;
                self.buf.clear();
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod test {
    // extern crate test;

    use super::*;
    use crate as atat;
    // use test::Bencher;
    use atat::Mode;
    use embedded_hal::serial::{self, Read};
    use heapless::{consts, spsc::Queue, String};

    #[test]
    fn no_response() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, mut c) = unsafe { REQ_Q.split() };
        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };
        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (_com_p, com_c) = unsafe { COM_Q.split() };

        let conf = Config::new(Mode::Timeout);
        let mut at_pars = IngressManager::new(p, urc_p, com_c, &conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write("AT+USORD=3,16\r\n".as_bytes());
        at_pars.parse_at();

        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write("OK\r\n".as_bytes());
        at_pars.parse_at();
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
        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (_com_p, com_c) = unsafe { COM_Q.split() };

        let conf = Config::new(Mode::Timeout);
        let mut at_pars = IngressManager::new(p, urc_p, com_c, &conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write("AT+USORD=3,16\r\n".as_bytes());
        at_pars.parse_at();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write("+USORD: 3,16,\"16 bytes of data\"\r\n".as_bytes());
        at_pars.parse_at();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write("OK\r\n".as_bytes());
        at_pars.parse_at();
        assert_eq!(at_pars.buf, String::<consts::U256>::from(""));
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
    fn urc() {
        static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
            Queue(heapless::i::Queue::u8());
        let (p, _c) = unsafe { REQ_Q.split() };
        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };
        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (_com_p, com_c) = unsafe { COM_Q.split() };

        let conf = Config::new(Mode::Timeout);
        let mut at_pars = IngressManager::new(p, urc_p, com_c, &conf);

        assert_eq!(at_pars.state, State::Idle);

        at_pars.write("+UUSORD: 3,16,\"16 bytes of data\"\r\n".as_bytes());
        at_pars.parse_at();
        assert_eq!(at_pars.buf, String::<consts::U256>::from(""));
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
        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (_com_p, com_c) = unsafe { COM_Q.split() };

        let conf = Config::new(Mode::Timeout);
        let mut at_pars = IngressManager::new(p, urc_p, com_c, &conf);

        for _ in 0..266 {
            at_pars.write(b"s");
        }
        at_pars.parse_at();

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
        static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
            Queue(heapless::i::Queue::u8());
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };
        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (_com_p, com_c) = unsafe { COM_Q.split() };

        let conf = Config::new(Mode::Timeout);
        let mut at_pars = IngressManager::new(p, urc_p, com_c, &conf);

        assert_eq!(at_pars.state, State::Idle);

        assert_eq!(at_pars.buf, String::<consts::U256>::from(""));
        at_pars.write("OK\r\n".as_bytes());
        at_pars.parse_at();

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
        static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
        let (_com_p, com_c) = unsafe { COM_Q.split() };

        let conf = Config::new(Mode::Timeout);
        let mut at_pars = IngressManager::new(p, urc_p, com_c, &conf);

        assert_eq!(at_pars.state, State::Idle);
        at_pars.write("AT+USORD=3,16\r\n".as_bytes());
        at_pars.parse_at();
        assert_eq!(at_pars.state, State::ReceivingResponse);

        at_pars.write("+USORD: 3,16,\"16 bytes of data\"\r\n".as_bytes());
        at_pars.parse_at();

        at_pars.write("ERROR\r\n".as_bytes());
        at_pars.parse_at();

        assert_eq!(at_pars.buf, String::<consts::U256>::from(""));
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

    // #[bench]
    // fn response_bench(b: &mut Bencher) {
    //     static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
    //         Queue(heapless::i::Queue::u8());
    //     let (p, _c) = unsafe { REQ_Q.split() };
    //     static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
    //         Queue(heapless::i::Queue::u8());
    //     let (urc_p, _urc_c) = unsafe { URC_Q.split() };
    //     static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
    //     let (_com_p, com_c) = unsafe { COM_Q.split() };

    //     let conf = Config::new(Mode::Timeout);
    //     let mut at_pars = IngressManager::new(p, urc_p, com_c, &conf);

    //     b.iter(|| {
    //                 at_pars.write("AT+USORD=3,16\r\nOK\r\n".as_bytes());
    //         at_pars.parse_at();
    //     });
    // }
}
