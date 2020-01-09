use embedded_hal::serial;

use heapless::{
    consts,
    spsc::{Consumer, Producer},
    ArrayLength, String, Vec,
};

use crate::buffer::Buffer;
use crate::error::Error as ATError;
use crate::traits::ATCommandInterface;
use crate::{MaxCommandLen, MaxResponseLines};

use log::{error, info, warn};

type CmdConsumer<C, N> = Consumer<'static, C, N, u8>;
type RespProducer<R, N> = Producer<'static, Result<R, ATError>, N, u8>;
type Queues<C, R, CmdQueueLen, RespQueueLen> =
    (CmdConsumer<C, CmdQueueLen>, RespProducer<R, RespQueueLen>);

pub struct ATParser<Serial, C, R, RxBufferLen, CmdQueueLen, RespQueueLen>
where
    Serial: serial::Write<u8> + serial::Read<u8>,
    C: ATCommandInterface<R>,
    RxBufferLen: ArrayLength<u8>,
    CmdQueueLen: ArrayLength<C>,
    RespQueueLen: ArrayLength<Result<R, ATError>>,
    R: core::fmt::Debug,
{
    serial: Serial,
    prev_cmd: Option<C>,
    rx_buf: Buffer<RxBufferLen>,
    cmd_c: CmdConsumer<C, CmdQueueLen>,
    resp_p: RespProducer<R, RespQueueLen>,
}

impl<Serial, Command, Response, RxBufferLen, CmdQueueLen, RespQueueLen>
    ATParser<Serial, Command, Response, RxBufferLen, CmdQueueLen, RespQueueLen>
where
    Serial: serial::Write<u8> + serial::Read<u8>,
    Command: ATCommandInterface<Response>,
    Response: core::fmt::Debug,
    RxBufferLen: ArrayLength<u8>,
    CmdQueueLen: ArrayLength<Command>,
    RespQueueLen: ArrayLength<Result<Response, ATError>>,
{
    pub fn new(
        mut serial: Serial,
        queues: Queues<Command, Response, CmdQueueLen, RespQueueLen>,
    ) -> Self {
        let (cmd_c, resp_p) = queues;

        block!(serial.flush()).ok();
        while serial.read().is_ok() {}

        Self {
            serial,
            prev_cmd: None,
            rx_buf: Buffer::new(),
            cmd_c,
            resp_p,
        }
    }

    pub fn release(self) -> (Serial, Queues<Command, Response, CmdQueueLen, RespQueueLen>) {
        (self.serial, (self.cmd_c, self.resp_p))
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

    fn notify_response(&mut self, response: Result<Response, ATError>) {
        if self.resp_p.ready() {
            self.resp_p.enqueue(response).ok();
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
                if let Some(resp) = Command::parse_unsolicited(&resp_line) {
                    self.rx_buf.remove_line(&resp_line);
                    self.notify_response(Ok(resp));
                }
            }
        }

        // Handle Send
        if let Some(cmd) = self.cmd_c.dequeue() {
            match self.send(cmd) {
                Ok(()) => (),
                Err(_e) => {
                    self.notify_response(Err(ATError::Write));
                }
            }
        }
    }

    /// Send an AT command to the module, extracting any relevant response
    fn send(
        &mut self,
        cmd: Command,
    ) -> Result<(), <Serial as embedded_hal::serial::Write<u8>>::Error> {
        let mut command = cmd.get_cmd();

        self.prev_cmd = Some(cmd);

        if !command.ends_with("\r\n") {
            command.push_str("\r\n").ok();
        }

        // Transmit the AT Command
        self.write_all(&command)
    }
}
