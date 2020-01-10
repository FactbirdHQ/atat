use embedded_hal::serial;

use heapless::{
    consts,
    spsc::{Consumer, Producer},
    ArrayLength, String, Vec,
};

use crate::buffer::Buffer;
use crate::error::Error as ATError;
use crate::traits::{ATCommandInterface, ATRequestType};
use crate::{MaxCommandLen, MaxResponseLines};
use crate::Response;

use log::{error, info, warn};

type CmdConsumer<Req, N> = Consumer<'static, Req, N, u8>;
type RespProducer<Res, N> = Producer<'static, Result<Res, ATError>, N, u8>;
type Queues<Req, Res, CmdQueueLen, RespQueueLen> =
    (CmdConsumer<Req, CmdQueueLen>, RespProducer<Res, RespQueueLen>);


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
        while serial.read().is_ok() {}

        Self {
            serial,
            prev_cmd: None,
            rx_buf: Buffer::new(),
            req_c,
            res_p,
        }
    }

    pub fn release(self) -> (Serial, Queues<Req, Response<Req>, CmdQueueLen, RespQueueLen>) {
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
                    error!("RXBuf is full!\r");
                }
            }
            Err(e) => match e {
                nb::Error::WouldBlock => (),
                nb::Error::Other(e) => {
                    error!("rx buffer error: {:?}\r", e);
                }
            },
        }
    }

    fn notify_response(&mut self, response: Result<Response<Req>, ATError>) {
        if self.res_p.ready() {
            self.res_p.enqueue(response).ok();
        } else {
            // TODO: Handle response queue not ready!
            warn!("Response queue is not ready!");
        }
    }

    fn write_all(
        &mut self,
        buffer: &str,
    ) -> Result<(), <Serial as embedded_hal::serial::Write<u8>>::Error> {
        for byte in buffer.bytes() {
            block!(self.serial.write(byte))?;
        }

        block!(self.serial.flush())?;

        Ok(())
    }

    pub fn spin(&mut self) {
        let mut lines: Vec<String<MaxCommandLen>, MaxResponseLines> = self
            .rx_buf
            .buffer
            .lines()
            .filter_map(|p| {
                if !p.is_empty() {
                    info!("{:?}", p);
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
                    let prev_command = prev_cmd.get_cmd();
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
        if let Some(cmd) = self.req_c.dequeue() {
            match self.write_all(cmd.get_bytes()) {
                Ok(()) => (),
                Err(_e) => {
                    self.notify_response(Err(ATError::Write));
                }
            }
            // if let Some(c) = cmd.try_get_cmd() {
            //     match self.send(c) {
            //         Ok(()) => (),
            //         Err(_e) => {
            //             self.notify_response(Err(ATError::Write));
            //         }
            //     }
            // }
        }
    }

}
    // Send an AT command to the module, extracting any relevant response
    // fn send(
    //     &mut self,
    //     cmd: Req::Command,
    // ) -> Result<(), <Serial as embedded_hal::serial::Write<u8>>::Error> {
    //     let mut command = cmd.get_cmd();

    //     self.prev_cmd = Some(cmd);

    //     if !command.ends_with("\r\n") {
    //         command.push_str("\r\n").ok();
    //     }

    //     // Transmit the AT Command
    //     self.write_all(&command)
    // }
