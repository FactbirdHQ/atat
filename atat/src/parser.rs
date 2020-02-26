use embedded_hal::serial;
use heapless::{consts, spsc::Producer, String};

use crate::buffer::Buffer;
use crate::error::{Error, Result};
use crate::Config;

type ResProducer = Producer<'static, Result<String<consts::U256>>, consts::U5, u8>;
type UrcProducer = Producer<'static, String<consts::U64>, consts::U10, u8>;

#[derive(Clone, PartialEq)]
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

                            let index = ind.0;
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
