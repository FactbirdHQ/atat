use embedded_hal::serial;

use heapless::{consts, spsc::Producer, ArrayLength, String};

use crate::buffer::Buffer;
use crate::error::{Error, Result};

type RespProducer = Producer<'static, Result<String<consts::U256>>, consts::U10, u8>;

#[derive(Clone, PartialEq)]
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
}

impl<Rx, RxBufferLen> ATParser<Rx, RxBufferLen>
where
    Rx: serial::Read<u8>,
    RxBufferLen: ArrayLength<u8>,
{
    pub fn new(mut rx: Rx, queue: RespProducer) -> Self {
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
            line_term_char: '\r',
            format_char: '\n',
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
                            if self.rx_buf.buffer.starts_with("AT")
                                && self.rx_buf.buffer.ends_with("\r\n")
                            {
                                self.state = State::ReceivingResponse;
                                self.rx_buf.buffer.clear();
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
                            if c as char == '\n' {
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

mod test {

    use super::*;
    use heapless::{String, consts, spsc::Queue};

    struct RxMock {
        cnt: u8,
        s: String<consts::U64>
    }

    impl RxMock {
        fn new(s: String<consts::U64>) -> Self {
            RxMock {
                cnt: 0,
                s
            }
        }
    }

    impl serial::Read<u8> for RxMock {
        type Error = ();

        fn read(&mut self) -> nb::Result<u8, Self::Error> {
            self.cnt += 1;
            Ok(self.s[self.cnt])
        }

    }

    #[test]
    fn test_sdnmg(){
        let (c, p) = Queue::u8().split();
        let parser = ATParser::new(RxMock::new(String::from("AT+TESST\r\n")), c);

        parser.handle_irq();
    }
}
