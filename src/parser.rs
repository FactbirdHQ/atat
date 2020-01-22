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

use log::{error, info, warn};

type CmdConsumer<Req, N> = Consumer<'static, Req, N, u8>;
type RespProducer<Res, N> = Producer<'static, Result<Res, ATError>, N, u8>;
type Queues<Req, Res, CmdQueueLen, RespQueueLen> = (
    CmdConsumer<Req, CmdQueueLen>,
    RespProducer<Res, RespQueueLen>,
);

#[derive(PartialEq)]
enum State<C> {
    Idle,
    WaitingResponse(C),
}

impl<C> State<C> {
    pub fn is_idle(&self) -> bool {
        match *self {
            State::Idle => true,
            _ => false,
        }
    }

    pub fn is_awaiting_response(&self) -> bool {
        match *self {
            State::WaitingResponse(_) => true,
            _ => false,
        }
    }
}

pub struct ATParser<Serial, Req, RxBufferLen, CmdQueueLen, RespQueueLen>
where
    Serial: serial::Write<u8> + serial::Read<u8>,
    Req: ATRequestType,
    Req::Command: ATCommandInterface + PartialEq,
    RxBufferLen: ArrayLength<u8>,
    CmdQueueLen: ArrayLength<Req>,
    RespQueueLen: ArrayLength<Result<Response<Req>, ATError>>,
    Response<Req>: core::fmt::Debug,
{
    serial: Serial,
    rx_buf: Buffer<RxBufferLen>,
    req_c: CmdConsumer<Req, CmdQueueLen>,
    res_p: RespProducer<Response<Req>, RespQueueLen>,
    state: State<Req::Command>,

    /// Command line termination character S3 (Default = '\r' [013])
    line_term_char: char,
    /// Response formatting character S4 (Default = '\n' [010])
    format_char: char,
    echo_enabled: bool,
}

impl<Serial, Req, RxBufferLen, CmdQueueLen, RespQueueLen>
    ATParser<Serial, Req, RxBufferLen, CmdQueueLen, RespQueueLen>
where
    Serial: serial::Write<u8> + serial::Read<u8>,
    Req: ATRequestType,
    Req::Command: ATCommandInterface + PartialEq,
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
            state: State::Idle,
            rx_buf: Buffer::new(),
            req_c,
            res_p,

            line_term_char: '\r',
            format_char: '\n',
            echo_enabled: true,
        }
    }

    pub fn set_line_termination_char(&mut self, c: char) {
        self.line_term_char = c;
    }

    pub fn set_format_char(&mut self, c: char) {
        self.format_char = c;
    }

    pub fn set_echo_enabled(&mut self, enabled: bool) {
        self.echo_enabled = enabled;
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
        match block!(self.serial.read()) {
            Ok(c) => {
                // FIXME: handle buffer being full
                if self.rx_buf.push(c).is_err() {
                    error!("RXBuf is full!\r");
                }
            }
            Err(e) => error!("{:?}\r", e),
        }
    }

    fn notify_response(&mut self, response: Result<Response<Req>, ATError>) {
        if self.res_p.ready() {
            self.res_p.enqueue(response).ok();
        } else {
            // FIXME: Handle response queue not ready!
            warn!("Response queue is not ready!\r");
        }
    }

    fn write_all(&mut self, buffer: &[u8]) -> Result<(), <Serial as serial::Write<u8>>::Error> {
        for &byte in buffer {
            block!(self.serial.write(byte))?;
        }
        block!(self.serial.flush())?;
        Ok(())
    }

    fn take_response(
        &mut self,
        lines: &Vec<String<MaxCommandLen>, MaxResponseLines>,
        final_result_code: String<consts::U7>,
    ) -> Vec<String<MaxCommandLen>, MaxResponseLines> {
        let full_response = lines
            .iter()
            .take_while(|&line| line.as_str() != final_result_code)
            .inspect(|line| self.rx_buf.remove_line(&line))
            .filter(|p| !p.is_empty())
            .cloned()
            .collect::<Vec<_, MaxResponseLines>>();

        self.rx_buf.remove_line(&final_result_code);

        full_response
    }

    pub fn spin(&mut self) {
        // TODO: Handle parsing Data Mode Packets + Extended Data Mode Packets

        if self.rx_buf.buffer.len() > 0 {
            let lines: Vec<String<MaxCommandLen>, MaxResponseLines> =
                self.rx_buf.at_lines(self.line_term_char, self.format_char);

            info!("Rx: {:?} - {:?}\r", lines, self.rx_buf.buffer);

            if self.state.is_awaiting_response() {
                // Information Text Response (ITR)
                if lines.iter().any(|line| line.as_str() == "ERROR") {
                    // Clean up the receive buffer
                    self.take_response(&lines, String::from("ERROR"));
                    self.state = State::Idle;
                    self.notify_response(Err(ATError::InvalidResponse));
                } else if lines.iter().any(|line| line.as_str() == "ABORTED") {
                    // Clean up the receive buffer
                    self.take_response(&lines, String::from("ABORTED"));
                    self.state = State::Idle;
                    self.notify_response(Err(ATError::Aborted));
                } else if lines.iter().any(|line| line.as_str() == "OK") {
                    info!("IM here\r");
                    let full_response = self.take_response(&lines, String::from("OK"));

                    info!("Received OK: {:?}\r", full_response);
                    if let State::WaitingResponse(prev_cmd) = &self.state {
                        let prev_command: String<MaxCommandLen> = prev_cmd.get_cmd();
                        info!("prev_cmd: {:?}\r", prev_command);
                        let filtered = full_response
                            .iter()
                            .filter(|line| !line.starts_with(prev_command.as_str()))
                            .cloned()
                            .collect();

                        let response = prev_cmd.parse_resp(&filtered);
                        self.notify_response(Ok(response));
                        self.state = State::Idle;
                    }
                }
                // } else {
                //     // Unsolicited Response Code (URC)
                //     if lines.len() > 0 {
                //         warn!("Unsolicited!\r");
                //         let resp_line = lines.pop().unwrap();
                //         if let Some(resp) = Req::Command::parse_unsolicited(&resp_line) {
                //             self.rx_buf.remove_line(&resp_line);
                //             self.notify_response(Ok(resp));
                // self.urc_handlers.for_each(|handler| handler(resp));
                //         }
                //     }
            }
        }

        // Handle Send
        if let Some(req) = self.req_c.dequeue() {
            let bytes: Vec<u8, consts::U1024> = req.get_bytes();

            // If we are currently sending an AT command, store it for parsing the response
            if let Some(cmd) = req.try_get_cmd() {
                self.state = State::WaitingResponse(cmd);
            }
            info!("Sending {:?}\r", bytes);

            match self.write_all(&bytes) {
                Ok(()) => (),
                Err(_e) => {
                    self.notify_response(Err(ATError::Write));
                }
            }
        }
    }
}
