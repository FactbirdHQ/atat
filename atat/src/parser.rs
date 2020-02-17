use embedded_hal::{serial, timer::CountDown};

use heapless::{consts, spsc::Producer, ArrayLength, String};

use crate::buffer::Buffer;
use crate::error::{Error, Result};
use crate::Config;

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
