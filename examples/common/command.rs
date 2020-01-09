//! AT Commands for ODIN-W2 module\
//! Following https://www.spezial.com/sites/default/files/odin-w2-atcommands_manual_ubx-14044127.pdf

use core::fmt::Write;
use heapless::{String, Vec};

use at::{utils, ATCommandInterface, MaxCommandLen, MaxResponseLines};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum DTRValue {
    /// DTR line is ignored
    Ignore = 0,
    /// (default and factory default value): upon an ON-to-OFF transition of circuit 108/2, the DCE enters
    /// command mode and issues and OK result code
    EnterCmdMode = 1,
    /// upon an ON-to-OFF transition of circuit 108/2, the DCE performs an orderly disconnect of all radio links
    /// and peer connections. No new connections will be established while circuit 108/2 remains OFF
    DisconnectLinks = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum DSRValue {
    /// sets DSR line to ON
    On = 0,
    /// (default value and factory default value): sets the DSR line to OFF in command mode and ON when not
    /// in command mode
    OffInCmdOtherwiseOn = 1,
    /// Sets the DSR line to ON in data mode when at least one remote peer is connected, all other cases it's
    /// set to off
    OnWhenConnectedPeers = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Command {
    // 5 General
    /// 5.1 AT Attention Command
    AT,
    /// 5.2 Manufacturer identification
    GetManufacturerId,
    /// 5.3 Model identification
    GetModelId,
    /// 5.4 Firmware version identification
    GetFWVersion,
    /// 5.5 Serial number
    GetSerialNum,
    /// Following commands are skipped, due to duplicates
    /// 5.6 Manufacturer identification (AT+GMI)
    /// 5.7 Model identification (AT+GMM)
    /// 5.8 Firmware version identification (AT+GMR)
    /// 5.10 Identification information I
    GetId,
    /// 5.11 Set greeting text\
    /// Set the greeting text\
    /// Configures and activates/deactivates the greeting text. The greeting text configuration's change will be
    /// applied at the subsequent boot. If active, the greeting text is shown at boot once, on any AT interface, if
    /// the module start up mode is set to command mode
    SetGreetingText {
        enable: bool,
        text: String<MaxCommandLen>,
    },
    /// Get the current greeting text
    GetGreetingText,

    // 6 System
    /// 6.1 Store current configuration
    Store,
    /// 6.2 Set to default configuration
    ResetDefault,
    /// 6.3 Set to factory defined configuration
    ResetFactory,
    /// 6.4 Circuit 108/2 (DTR) behavior
    SetDTR { value: DTRValue },
    /// 6.5 DSR Overide
    SetDSR { value: DSRValue },
    /// 6.6 ATE Echo On/Off\
    /// This command configures whether or not the unit echoes characters received from the DTE when in Command Mode.
    SetEcho { enable: bool },
    /// Read current echo setting
    GetEcho,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Response {
    ManufacturerId {
        id: String<MaxCommandLen>,
    },
    ModelId {
        id: String<MaxCommandLen>,
    },
    FWVersion {
        version: String<MaxCommandLen>,
    },
    SerialNum {
        serial: String<MaxCommandLen>,
    },
    Id {
        id: String<MaxCommandLen>,
    },
    GreetingText {
        enable: bool,
        text: String<MaxCommandLen>,
    },

    DTR {
        value: DTRValue,
    },
    DSR {
        value: DSRValue,
    },
    Echo {
        enable: bool,
    },
    None,
}

impl ATCommandInterface<Response> for Command {
    fn get_cmd(&self) -> String<MaxCommandLen> {
        let mut buffer = String::new();
        match self {
            Command::AT => String::from("AT"),
            Command::GetManufacturerId => String::from("AT+CGMI"),
            Command::GetModelId => String::from("AT+CGMM"),
            Command::GetFWVersion => String::from("AT+CGMR"),
            Command::GetSerialNum => String::from("AT+CGSN"),
            Command::GetId => String::from("ATI9"),
            Command::SetGreetingText {
                ref enable,
                ref text,
            } => {
                if *enable {
                    if text.len() > 49 {
                        // TODO: Error!
                    }
                    write!(buffer, "AT+CSGT={},{}", *enable as u8, text).unwrap();
                } else {
                    write!(buffer, "AT+CSGT={}", *enable as u8).unwrap();
                }
                buffer
            }
            Command::GetGreetingText => String::from("AT+CSGT?"),
            Command::Store => String::from("AT&W"),
            Command::ResetDefault => String::from("ATZ0"),
            Command::ResetFactory => String::from("AT+UFACTORY"),
            Command::SetDTR { ref value } => {
                write!(buffer, "AT&D{}", *value as u8).unwrap();
                buffer
            }
            Command::SetDSR { ref value } => {
                write!(buffer, "AT&S{}", *value as u8).unwrap();
                buffer
            }
            Command::SetEcho { ref enable } => {
                write!(buffer, "ATE{}", *enable as u8).unwrap();
                buffer
            }
            Command::GetEcho => String::from("ATE?"),
        }
    }

    fn parse_resp(
        &self,
        response_lines: &mut Vec<String<MaxCommandLen>, MaxResponseLines>,
    ) -> Response {
        if response_lines.is_empty() {
            return Response::None;
        }
        let mut responses: Vec<Vec<&str, MaxResponseLines>, MaxResponseLines> =
            utils::split_parameterized_resp(response_lines);

        let response = responses.pop().unwrap();

        match *self {
            Command::AT => Response::None,
            Command::GetManufacturerId => Response::ManufacturerId {
                id: String::from(response[0]),
            },
            _ => Response::None,
        }
    }

    fn parse_unsolicited(_response_line: &str) -> Result<Response, ()> {
        Ok(Response::None)
    }
}
