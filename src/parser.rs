use embedded_hal::serial;

use heapless::{
    consts,
    spsc::{Consumer, Producer},
    String, Vec,
};

use crate::buffer::Buffer;
use crate::error::Error as ATError;
use crate::traits::ATCommandInterface;
use crate::{MaxCommandLen, MaxResponseLines};

type CmdConsumer<C> = Consumer<'static, C, consts::U4, u8>;
type RespProducer<R> = Producer<'static, Result<R, ATError>, consts::U4, u8>;

pub struct ATParser<Serial, C, R>
where
    Serial: serial::Write<u8> + serial::Read<u8>,
    C: ATCommandInterface<R>,
    R: core::fmt::Debug,
{
    serial: Serial,
    prev_cmd: Option<C>,
    rx_buf: Buffer,
    cmd_c: CmdConsumer<C>,
    resp_p: RespProducer<R>,
}

impl<Serial, Command, Response> ATParser<Serial, Command, Response>
where
    Serial: serial::Write<u8> + serial::Read<u8>,
    Command: ATCommandInterface<Response>,
    Response: core::fmt::Debug,
{
    pub fn new(serial: Serial, queues: (CmdConsumer<Command>, RespProducer<Response>)) -> Self {
        let (cmd_c, resp_p) = queues;
        Self {
            serial,
            prev_cmd: None,
            rx_buf: Buffer::new(),
            cmd_c,
            resp_p,
        }
    }

    pub fn release(self) -> (Serial, (CmdConsumer<Command>, RespProducer<Response>)) {
        (self.serial, (self.cmd_c, self.resp_p))
    }

    pub fn handle_irq(&mut self) {
        match self.serial.read() {
            Ok(c) => {
                // FIXME: handle buffer being full
                if self.rx_buf.push(c).is_err() {
                    // error!("RXBuf is full!\r");
                }
            }
            Err(e) => {
                match e {
                    nb::Error::WouldBlock => {
                        // no data available
                    }
                    nb::Error::Other(_) => {
                        // info!("rx buffer error\r");
                    }
                }
            }
        }
    }

    fn notify_response(&mut self, response: Result<Response, ATError>) {
        if self.resp_p.ready() {
            self.resp_p.enqueue(response).ok();
        } else {
            // TODO: Handle response queue not ready!
            // warn!("Response queue is not ready!");
        }
    }

    fn write_all(
        &mut self,
        buffer: &str,
    ) -> Result<(), <Serial as embedded_hal::serial::Write<u8>>::Error> {
        for byte in buffer.bytes() {
            block!(self.serial.write(byte))?;
        }

        Ok(())
    }

    pub fn spin(&mut self) {
        if let Some((response, remainder)) = self.rx_buf.split_response() {
            self.rx_buf = remainder;
            if let Ok(resp) = core::str::from_utf8(&response.buffer[0..response.index]) {
                if resp == "ERROR" {
                    // Fail fast
                    self.notify_response(Err(ATError::ParseString));
                } else if let Some(prev_cmd) = &self.prev_cmd {
                    let prev_command = prev_cmd.get_cmd();
                    prev_command.starts_with("whaat");
                    let mut lines: Vec<String<MaxCommandLen>, MaxResponseLines> = resp
                        .lines()
                        .filter(|ref p| !p.starts_with(prev_command.as_str()) || !p.is_empty())
                        .map(String::from)
                        .collect();

                    let response = prev_cmd.parse_resp(&mut lines);
                    self.notify_response(Ok(response));
                    self.prev_cmd = None;
                }
            }
        } else if let Some((response, remainder)) = self.rx_buf.split_line() {
            self.rx_buf = remainder;
            // Attempt to take a single full line, as there might be an unsolicited message response
            if let Ok(resp) = core::str::from_utf8(&response.buffer[0..response.index]) {
                self.notify_response(Ok(Command::parse_unsolicited(resp)));
            }
        }
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
