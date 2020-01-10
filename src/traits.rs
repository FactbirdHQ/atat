use crate::error::Error;
use crate::{MaxCommandLen, MaxResponseLines};
use embedded_hal::timer::CountDown;
use heapless::{String, Vec};

/// Trait to be implemented by device driver crates.
///
/// # Examples
///
/// ```
/// #[derive(Debug, Clone)]
/// enum Command {
///   AT,
///   GetSerialNum,
///   GetUMSM,
///   GetCSGT,
///   SetDefaultPeer {
///     peer_id: u8,
///     url: String<MaxCommandLen>,
///     connect_scheme: u8,
///   },
/// }

/// #[derive(Debug, Clone, PartialEq)]
/// enum Response {
///   None,
///   SerialNum { serial: String<MaxCommandLen> },
///   UMSM { start_mode: u8 },
///   CSGT { mode: u8, text: String<MaxCommandLen> },

///   // Unsolicited responses
///   PeerDisconnected { peer_handle: u8 },
/// }

/// impl ATCommandInterface<Response> for Command {
///   fn get_cmd(&self) -> String<MaxCommandLen> {
///     let mut buffer = String::new();
///     match self {
///       Command::AT => String::from("AT"),
///       Command::GetUMSM => String::from("AT+UMSM?\r\n"),
///       Command::GetCSGT => String::from("AT+CSGT?\r\n"),
///       Command::GetSerialNum => String::from("AT+CGSN"),
///       Command::SetDefaultPeer {
///         ref peer_id,
///         ref url,
///         ref connect_scheme,
///       } => {
///         write!(
///           buffer,
///           "AT+UDDRP={},{},{}",
///           peer_id, url, *connect_scheme as u8
///         )
///         .unwrap();
///         buffer
///       }
///     }
///   }
///   fn parse_resp(&self, response_lines: &mut Vec<String<MaxCommandLen>, MaxResponseLines>) -> Response {
///     if response_lines.is_empty() {
///       return Response::None;
///     }
///     let mut responses: Vec<Vec<&str, MaxResponseLines>, MaxResponseLines> = utils::split_parameterized_resp(response_lines);

///     let response = responses.pop().unwrap();

///     match *self {
///       Command::AT => Response::None,
///       Command::GetUMSM => Response::UMSM {
///         start_mode: response[0].parse::<u8>().unwrap(),
///       },
///       Command::GetCSGT => Response::CSGT {
///         mode: response[0].parse::<u8>().unwrap(),
///         text: String::from(response[1]),
///       },
///       Command::GetSerialNum => Response::SerialNum {
///         serial: String::from(response[0]),
///       },
///       Command::SetDefaultPeer { .. } => Response::None,
///     }
///   }

///   fn parse_unsolicited(response_line: &str) -> Response {
///     let (cmd, parameters) = utils::split_parameterized_unsolicited(response_line);

///     match cmd {
///       "+UUDPD" => Response::PeerDisconnected {
///         peer_handle: parameters[0].parse::<u8>().unwrap(),
///       },
///       _ => Response::None,
///     }
///   }
/// }
/// ```
pub trait ATCommandInterface {
    type Response;

    fn get_cmd(&self) -> String<MaxCommandLen>;
    fn parse_resp(&self, response_lines: &mut Vec<String<MaxCommandLen>, MaxResponseLines>) -> Self::Response;
    fn parse_unsolicited(response_line: &str) -> Option<Self::Response>;
}

pub trait ATRequestType {
    type Command;

    fn try_get_cmd(self) -> Option<Self::Command>;
    fn get_bytes(&self) -> &str;
}

pub trait ATInterface<T: CountDown, Command, Response> {
    fn send(&mut self, cmd: Command) -> Result<Response, Error>;

    fn send_timeout(&mut self, cmd: Command, timeout: T::Time) -> Result<Response, Error>;

    fn wait_response(&mut self) -> Result<Response, Error>;

    fn peek_response(&mut self) -> &Result<Response, Error>;

    fn wait_response_timeout(&mut self, timeout: T::Time) -> Result<Response, Error>;
}
