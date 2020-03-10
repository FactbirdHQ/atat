use heapless::{
    consts,
    spsc::{Consumer, Producer},
    String,
    ArrayLength,
};

use crate::error::{Error, Result};
use crate::Command;
use crate::Config;

type ResProducer = Producer<'static, Result<String<consts::U256>>, consts::U5, u8>;
type UrcProducer = Producer<'static, String<consts::U64>, consts::U10, u8>;
type ComConsumer = Consumer<'static, Command, consts::U3, u8>;

#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Idle,
    ReceivingResponse,
}

pub struct IngressManager {
    rb: String<consts::U256>,
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
        // while rx.read().is_ok() {
        //     // Fix for unit tests!
        //     if let Ok(c) = rx.read() {
        //         if c == 0xFF {
        //             break;
        //         }
        //     }
        // }

        Self {
            state: State::Idle,
            rb: String::new(),
            res_p,
            urc_p,
            com_c,
            line_term_char: config.line_term_char,
            format_char: config.format_char,
            echo_enabled: config.at_echo_enabled,
        }
    }

    /// Write data into the internal ring buffer
    /// raw bytes being the core type allows the ingress manager to
    /// be abstracted over the communication medium,
    /// in theory if we setup usb serial, we could have two ingress managers
    /// working in harmony
    pub fn write(&mut self, data: &[u8]) {
        log::info!("IT WRITES\r");
        for byte in data {
            match self.rb.push(*byte as char) {
                Ok(_) => {},
                Err(e) => panic!("Ring buffer overflow by {:?} bytes", e)
            }
        }
    }

    fn take_buffer<L: ArrayLength<u8>>(&mut self, index: usize) -> String<L> {
        let mut result = String::new();
        let mut return_string: String<L> = String::new();
        return_string
            .push_str(unsafe { self.rb.get_unchecked(0..index).trim() })
            .ok();
        result
            .push_str(unsafe { self.rb.get_unchecked(index..self.rb.len()) })
            .ok();
        self.rb = result;
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

    fn handle_com(&mut self) {
        if let Some(com) = self.com_c.dequeue() {
            match com {
                Command::ClearBuffer => {
                    self.state = State::Idle;
                    self.rb.clear()
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
        match self.state {
            State::Idle => {
                if self.echo_enabled && self.rb.starts_with("AT") {
                    self.state = State::ReceivingResponse;
                    self.rb.clear();
                } else if !self.echo_enabled {
                    unimplemented!("Disabling AT echo is currently unsupported");
                } else if self.rb.starts_with("+") {
                    let resp = self.take_buffer(self.rb.len());
                    self.notify_urc(resp);
                    self.rb.clear();
                } else {
                    self.rb.clear();
                }
            }
            State::ReceivingResponse => {
                let (index, err) = if let Some(index) =
                    self.rb.rmatch_indices("OK\r\n").next()
                {
                    (index.0, None)
                } else if let Some(index) =
                    self.rb.rmatch_indices("ERROR\r\n").next()
                {
                    #[cfg(not(feature = "error-message"))]
                    let err = Error::InvalidResponse;
                    #[cfg(feature = "error-message")]
                    let err =
                        Error::InvalidResponseWithMessage(self.rb.clone());

                    (index.0, Some(err))
                } else {
                    return;
                };

                let resp = self.take_buffer(index);
                self.notify_response(match err {
                    None => Ok(resp),
                    Some(e) => Err(e),
                });

                self.state = State::Idle;
                self.rb.clear();
            }
        }
    }

    // pub fn handle_irq(&mut self) -> bool {
    //     if let Ok(c) = block!(self.rx.read()) {
    //         self.handle_com();
    //         if self.rb.push(c as char).is_err() {
    //             // Notify error response, and reset rb
    //             self.notify_response(Err(Error::Overflow));
    //             self.rb.clear();
    //         } else {
    //             if c as char == self.format_char {
    //                 return true
    //             }
    //         }
    //     }
    //     false
    // }
}

// #[cfg(test)]
// #[cfg_attr(tarpaulin, skip)]
// mod test {
//     extern crate test;

//     use super::*;
//     use crate as atat;
//     use test::Bencher;
//     use atat::Mode;
//     use heapless::{consts, spsc::Queue, String};
//     use nb;

//     struct RxMock {
//         cnt: usize,
//         s: String<consts::U512>,
//         cleared: bool,
//     }

//     impl RxMock {
//         fn new(s: String<consts::U512>) -> Self {
//             RxMock {
//                 cnt: 0,
//                 s,
//                 cleared: false,
//             }
//         }
//     }

//     impl serial::Read<u8> for RxMock {
//         type Error = ();

//         fn read(&mut self) -> nb::Result<u8, Self::Error> {
//             if self.cleared {
//                 if self.cnt >= self.s.len() {
//                     Err(nb::Error::Other(()))
//                 } else {
//                     let r = Ok(self.s.clone().into_bytes()[self.cnt]);
//                     self.cnt += 1;
//                     r
//                 }
//             } else {
//                 self.cleared = true;
//                 println!("Cleared");
//                 Err(nb::Error::Other(()))
//             }
//         }
//     }

//     #[test]
//     fn no_response() {
//         static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (p, mut c) = unsafe { REQ_Q.split() };
//         static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (urc_p, _urc_c) = unsafe { URC_Q.split() };
//         static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
//         let (_com_p, com_c) = unsafe { COM_Q.split() };

//         let rx_mock = RxMock::new(String::from("AT+USORD=3,16\r\nOK\r\n"));
//         let conf = Config::new(Mode::Timeout);
//         let mut at_pars: IngressManager<RxMock> = IngressManager::new(rx_mock, p, urc_p, com_c, &conf);

//         assert_eq!(at_pars.state, State::Idle);
//         for _ in 0.."AT+USORD=3,16\r\n".len() {
//             at_pars.handle_irq();
//         }
//         assert_eq!(at_pars.state, State::ReceivingResponse);

//         for _ in 0.."OK\r\n".len() {
//             at_pars.handle_irq();
//         }
//         assert_eq!(at_pars.state, State::Idle);

//         if let Some(result) = c.dequeue() {
//             match result {
//                 Ok(resp) => {
//                     assert_eq!(resp, String::<consts::U256>::from(""));
//                 }
//                 Err(e) => panic!("Dequeue Some error: {:?}", e),
//             };
//         } else {
//             panic!("Dequeue None.")
//         }
//     }

//     #[test]
//     fn response() {
//         static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (p, mut c) = unsafe { REQ_Q.split() };
//         static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (urc_p, _urc_c) = unsafe { URC_Q.split() };
//         static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
//         let (_com_p, com_c) = unsafe { COM_Q.split() };

//         let rx_mock = RxMock::new(String::from(
//             "AT+USORD=3,16\r\n+USORD: 3,16,\"16 bytes of data\"\r\nOK\r\n",
//         ));
//         let conf = Config::new(Mode::Timeout);
//         let mut at_pars: IngressManager<RxMock> = IngressManager::new(rx_mock, p, urc_p, com_c, &conf);

//         assert_eq!(at_pars.state, State::Idle);
//         for _ in 0.."AT+USORD=3,16\r\n".len() {
//             at_pars.handle_irq();
//         }
//         assert_eq!(at_pars.state, State::ReceivingResponse);

//         for _ in 0.."+USORD: 3,16,\"16 bytes of data\"\r\n".len() {
//             at_pars.handle_irq();
//         }

//         for _ in 0.."OK\r\n".len() {
//             at_pars.handle_irq();
//         }
//         assert_eq!(at_pars.rb, String::<consts::U256>::from(""));
//         assert_eq!(at_pars.state, State::Idle);

//         if let Some(result) = c.dequeue() {
//             match result {
//                 Ok(resp) => {
//                     assert_eq!(
//                         resp,
//                         String::<consts::U256>::from("+USORD: 3,16,\"16 bytes of data\"")
//                     );
//                 }
//                 Err(e) => panic!("Dequeue Some error: {:?}", e),
//             };
//         } else {
//             panic!("Dequeue None.")
//         }
//     }

//     #[test]
//     fn urc() {
//         static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (p, _c) = unsafe { REQ_Q.split() };
//         static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (urc_p, _urc_c) = unsafe { URC_Q.split() };
//         static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
//         let (_com_p, com_c) = unsafe { COM_Q.split() };

//         let rx_mock = RxMock::new(String::from("+UUSORD: 3,16,\"16 bytes of data\"\r\n"));
//         let conf = Config::new(Mode::Timeout);
//         let mut at_pars: IngressManager<RxMock> = IngressManager::new(rx_mock, p, urc_p, com_c, &conf);

//         assert_eq!(at_pars.state, State::Idle);

//         for _ in 0.."+UUSORD: 3,16,\"16 bytes of data\"\r\n".len() {
//             at_pars.handle_irq();
//         }
//         assert_eq!(at_pars.rb, String::<consts::U256>::from(""));
//         assert_eq!(at_pars.state, State::Idle);
//     }

//     #[test]
//     fn overflow() {
//         static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (p, mut c) = unsafe { REQ_Q.split() };
//         static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (urc_p, _urc_c) = unsafe { URC_Q.split() };
//         static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
//         let (_com_p, com_c) = unsafe { COM_Q.split() };

//         let mut t = String::<consts::U512>::new();

//         for _ in 0..266 {
//             t.push('s').ok();
//         }

//         let rx_mock = RxMock::new(t);
//         let conf = Config::new(Mode::Timeout);
//         let mut at_pars: IngressManager<RxMock> = IngressManager::new(rx_mock, p, urc_p, com_c, &conf);

//         for _ in 0..266 {
//             at_pars.handle_irq();
//         }

//         if let Some(result) = c.dequeue() {
//             match result {
//                 Err(e) => assert_eq!(e, Error::Overflow),
//                 Ok(resp) => {
//                     panic!("Dequeue Ok: {:?}", resp);
//                 }
//             };
//         } else {
//             panic!("Dequeue None.")
//         }
//     }

//     #[test]
//     fn read_error() {
//         static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (p, _c) = unsafe { REQ_Q.split() };
//         let rx_mock = RxMock::new(String::from("OK\r\n"));
//         static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (urc_p, _urc_c) = unsafe { URC_Q.split() };
//         static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
//         let (_com_p, com_c) = unsafe { COM_Q.split() };

//         let conf = Config::new(Mode::Timeout);
//         let mut at_pars: IngressManager<RxMock> = IngressManager::new(rx_mock, p, urc_p, com_c, &conf);

//         assert_eq!(at_pars.state, State::Idle);

//         assert_eq!(at_pars.rb, String::<consts::U256>::from(""));
//         for _ in 0.."OK\r\n".len() + 1 {
//             at_pars.handle_irq();
//         }

//         at_pars.rx.s.push('s').ok();
//         assert_eq!(at_pars.state, State::Idle);
//     }

//     #[test]
//     fn error_response() {
//         static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (p, mut c) = unsafe { REQ_Q.split() };

//         static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (urc_p, _urc_c) = unsafe { URC_Q.split() };
//         static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
//         let (_com_p, com_c) = unsafe { COM_Q.split() };

//         let rx_mock = RxMock::new(String::from(
//             "AT+USORD=3,16\r\n+USORD: 3,16,\"16 bytes of data\"\r\nERROR\r\n",
//         ));
//         let conf = Config::new(Mode::Timeout);
//         let mut at_pars: IngressManager<RxMock> = IngressManager::new(rx_mock, p, urc_p, com_c, &conf);

//         assert_eq!(at_pars.state, State::Idle);
//         for _ in 0.."AT+USORD=3,16\r\n".len() {
//             at_pars.handle_irq();
//         }
//         assert_eq!(at_pars.state, State::ReceivingResponse);

//         for _ in 0.."+USORD: 3,16,\"16 bytes of data\"\r\n".len() {
//             at_pars.handle_irq();
//         }

//         for _ in 0.."ERROR\r\n".len() {
//             at_pars.handle_irq();
//         }

//         assert_eq!(at_pars.rb, String::<consts::U256>::from(""));
//         assert_eq!(at_pars.state, State::Idle);

//         #[cfg(feature = "error-message")]
//         let expectation = Error::InvalidResponseWithMessage(String::from(
//             "+USORD: 3,16,\"16 bytes of data\"\r\nERROR\r\n",
//         ));
//         #[cfg(not(feature = "error-message"))]
//         let expectation = Error::InvalidResponse;

//         if let Some(result) = c.dequeue() {
//             match result {
//                 Err(e) => assert_eq!(e, expectation),
//                 Ok(resp) => {
//                     panic!("Dequeue Ok: {:?}", resp);
//                 }
//             };
//         } else {
//             panic!("Dequeue None.")
//         }
//     }

//     #[bench]
//     fn response_bench(b: &mut Bencher) {
//         static mut REQ_Q: Queue<Result<String<consts::U256>>, consts::U5, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (p, _c) = unsafe { REQ_Q.split() };
//         static mut URC_Q: Queue<String<consts::U64>, consts::U10, u8> =
//             Queue(heapless::i::Queue::u8());
//         let (urc_p, _urc_c) = unsafe { URC_Q.split() };
//         static mut COM_Q: Queue<Command, consts::U3, u8> = Queue(heapless::i::Queue::u8());
//         let (_com_p, com_c) = unsafe { COM_Q.split() };

//         let rx_mock = RxMock::new(String::from("AT+USORD=3,16\r\nOK\r\n"));
//         let conf = Config::new(Mode::Timeout);
//         let mut at_pars: IngressManager<RxMock> = IngressManager::new(rx_mock, p, urc_p, com_c, &conf);

//         b.iter(|| {
//             for _ in 0.."AT+USORD=3,16\r\nOK\r\n".len() {
//                 at_pars.handle_irq();
//             }
//         });
//     }
// }
