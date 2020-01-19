use embedded_hal::serial;

use heapless::{
    consts,
    spsc::{Consumer, Producer},
    ArrayLength, String, Vec,
};

use crate::buffer::Buffer;
use crate::error::Error as ATError;
use crate::traits::{ATCommandInterface, ATRequestType};
use crate::Response;
use crate::{MaxCommandLen, MaxResponseLines};

#[cfg(test)]
use log::{error, trace, warn};

type CmdConsumer<Req, N> = Consumer<'static, Req, N, u8>;
type RespProducer<Res, N> = Producer<'static, Result<Res, ATError>, N, u8>;
type Queues<Req, Res, CmdQueueLen, RespQueueLen> = (
    CmdConsumer<Req, CmdQueueLen>,
    RespProducer<Res, RespQueueLen>,
);

pub struct ATParser<Serial, Req, RxBufferLen, CmdQueueLen, RespQueueLen>
where
    Serial: serial::Write<u8> + serial::Read<u8>,
    Req: ATRequestType,
    Req::Command: ATCommandInterface,
    RxBufferLen: ArrayLength<u8>,
    CmdQueueLen: ArrayLength<Req>,
    RespQueueLen: ArrayLength<Result<Response<Req>, ATError>>,
    Response<Req>: core::fmt::Debug,
{
    serial: Serial,
    prev_cmd: Option<Req::Command>,
    rx_buf: Buffer<RxBufferLen>,
    req_c: CmdConsumer<Req, CmdQueueLen>,
    res_p: RespProducer<Response<Req>, RespQueueLen>,
}

impl<Serial, Req, RxBufferLen, CmdQueueLen, RespQueueLen>
    ATParser<Serial, Req, RxBufferLen, CmdQueueLen, RespQueueLen>
where
    Serial: serial::Write<u8> + serial::Read<u8>,
    Req: ATRequestType,
    Req::Command: ATCommandInterface,
    Response<Req>: core::fmt::Debug,
    RxBufferLen: ArrayLength<u8>,
    CmdQueueLen: ArrayLength<Req>,
    RespQueueLen: ArrayLength<Result<Response<Req>, ATError>>,
{
    pub fn new(
        mut serial: Serial,
        queues: Queues<Req, Response<Req>, CmdQueueLen, RespQueueLen>,
    ) -> Self {
        let (req_c, res_p) = queues;

        block!(serial.flush()).ok();
        while serial.read().is_ok() {
            // Fix for unit tests!
            if let Ok(c) = serial.read() {
                if c == 0xFF {
                    break;
                }
            }
        }

        Self {
            serial,
            prev_cmd: None,
            rx_buf: Buffer::new(),
            req_c,
            res_p,
        }
    }

    pub fn release(
        self,
    ) -> (
        Serial,
        Queues<Req, Response<Req>, CmdQueueLen, RespQueueLen>,
    ) {
        (self.serial, (self.req_c, self.res_p))
    }

    pub fn handle_irq(&mut self)
    where
        <Serial as serial::Read<u8>>::Error: core::fmt::Debug,
    {
        match self.serial.read() {
            Ok(c) => {
                // FIXME: handle buffer being full
                if self.rx_buf.push(c).is_err() {
                    #[cfg(test)]
                    error!("RXBuf is full!\r");
                }
            }
            Err(e) => match e {
                nb::Error::WouldBlock => (),
                nb::Error::Other(e) => {
                    #[cfg(test)]
                    error!("rx buffer error: {:?}\r", e);
                    #[cfg(not(test))] // Silence unused variable warning
                    let _ = e;
                }
            },
        }
    }

    fn notify_response(&mut self, response: Result<Response<Req>, ATError>) {
        if self.res_p.ready() {
            self.res_p.enqueue(response).ok();
        } else {
            // TODO: Handle response queue not ready!
            #[cfg(test)]
            warn!("Response queue is not ready!");
        }
    }

    fn write_all(&mut self, buffer: &[u8]) -> Result<(), <Serial as serial::Write<u8>>::Error> {
        for &byte in buffer {
            block!(self.serial.write(byte))?;
        }
        block!(self.serial.flush())?;
        Ok(())
    }

    pub fn spin(&mut self) {
        // TODO: Handle parsing Data Mode Packets + Extended Data Mode Packets

        let mut lines: Vec<String<MaxCommandLen>, MaxResponseLines> = self
            .rx_buf
            .buffer
            .lines()
            .filter_map(|p| {
                if !p.is_empty() {
                    #[cfg(test)]
                    trace!("{:?}", p);
                    Some(String::from(p))
                } else {
                    None
                }
            })
            .collect();

        if self.prev_cmd.is_some() {
            // Solicited

            if lines.iter().any(|line| line.as_str() == "ERROR") {
                self.notify_response(Err(ATError::InvalidResponse));
            } else if lines.iter().any(|line| line.as_str() == "OK") {
                let full_response = lines
                    .iter()
                    .take_while(|&line| line.as_str() != "OK" && line.as_str() != "ERROR")
                    .cloned()
                    .inspect(|line| self.rx_buf.remove_line(&line))
                    .collect::<Vec<_, MaxResponseLines>>();

                // FIXME
                self.rx_buf.remove_line(&String::<consts::U2>::from("OK"));

                if let Some(prev_cmd) = &self.prev_cmd {
                    let prev_command: String<MaxCommandLen> = prev_cmd.get_cmd();
                    let mut filtered = full_response
                        .iter()
                        .filter(|line| !line.starts_with(prev_command.as_str()))
                        .cloned()
                        .collect();

                    let response = prev_cmd.parse_resp(&mut filtered);
                    self.notify_response(Ok(response));
                    self.prev_cmd = None;
                }
            }
        } else {
            // Unsolicited
            if lines.len() > 0 {
                let resp_line = lines.pop().unwrap();
                if let Some(resp) = Req::Command::parse_unsolicited(&resp_line) {
                    self.rx_buf.remove_line(&resp_line);
                    self.notify_response(Ok(resp));
                }
            }
        }

        // Handle Send
        if let Some(req) = self.req_c.dequeue() {
            let bytes: Vec<u8, consts::U1024> = req.get_bytes();

            // If we are currently sending an AT command, store it for parsing the response
            if let Some(cmd) = req.try_get_cmd() {
                self.prev_cmd = Some(cmd);
            }

            match self.write_all(&bytes) {
                Ok(()) => (),
                Err(_e) => {
                    self.notify_response(Err(ATError::Write));
                }
            }
        }
    }
}
