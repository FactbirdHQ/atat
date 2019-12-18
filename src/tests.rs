
extern crate env_logger;
extern crate std;

use super::*;
use core::fmt::Write;
use embedded_hal_mock::serial::{Mock as SerialMock, Transaction as SerialTransaction};

use heapless::{
    String,
    Vec,
    spsc::Queue,
    consts,
};
#[allow(unused_imports)]
use log::{error, info, warn};

use crate::error::Error as ATError;
use crate::traits::ATCommandInterface;
use crate::utils;
use crate::{MaxCommandLen, MaxResponseLines};

use env_logger::Env;
use std::sync::Once;

static INIT: Once = Once::new();

fn setup_log() {
  INIT.call_once(|| {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
      .is_test(true)
      .init();
  });
}

#[derive(Debug, Clone)]
enum TestCommand {
  AT,
  GetSerialNum,
  GetUMSM,
  GetCSGT,
  SetDefaultPeer {
    peer_id: u8,
    url: String<MaxCommandLen>,
    connect_scheme: u8,
  },
}

#[derive(Debug, Clone, PartialEq)]
enum TestResponse {
  None,
  SerialNum { serial: String<MaxCommandLen> },
  UMSM { start_mode: u8 },
  CSGT { mode: u8, text: String<MaxCommandLen> },

  // Unsolicited responses
  PeerDisconnected { peer_handle: u8 },
}

impl ATCommandInterface<TestResponse> for TestCommand {
  fn get_cmd(&self) -> String<MaxCommandLen> {
    let mut buffer = String::new();
    match self {
      TestCommand::AT => String::from("AT"),
      TestCommand::GetUMSM => String::from("AT+UMSM?\r\n"),
      TestCommand::GetCSGT => String::from("AT+CSGT?\r\n"),
      TestCommand::GetSerialNum => String::from("AT+CGSN"),
      TestCommand::SetDefaultPeer {
        ref peer_id,
        ref url,
        ref connect_scheme,
      } => {
        write!(
          buffer,
          "AT+UDDRP={},{},{}",
          peer_id, url, *connect_scheme as u8
        )
        .unwrap();
        buffer
      }
    }
  }
  fn parse_resp(&self, response_lines: &mut Vec<String<MaxCommandLen>, MaxResponseLines>) -> TestResponse {
    if response_lines.is_empty() {
      return TestResponse::None;
    }
    let mut responses: Vec<Vec<&str, MaxResponseLines>, MaxResponseLines> = utils::split_parameterized_resp(response_lines);

    let response = responses.pop().unwrap();

    match *self {
      TestCommand::AT => TestResponse::None,
      TestCommand::GetUMSM => TestResponse::UMSM {
        start_mode: response[0].parse::<u8>().unwrap(),
      },
      TestCommand::GetCSGT => TestResponse::CSGT {
        mode: response[0].parse::<u8>().unwrap(),
        text: String::from(response[1]),
      },
      TestCommand::GetSerialNum => TestResponse::SerialNum {
        serial: String::from(response[0]),
      },
      TestCommand::SetDefaultPeer { .. } => TestResponse::None,
    }
  }

  fn parse_unsolicited(response_line: &str) -> TestResponse {
    let (cmd, parameters) = utils::split_parameterized_unsolicited(response_line);

    match cmd {
      "+UUDPD" => TestResponse::PeerDisconnected {
        peer_handle: parameters[0].parse::<u8>().unwrap(),
      },
      _ => TestResponse::None,
    }
  }
}

macro_rules! setup {
  ($expectations: expr) => {{
    setup_log();

    let wifi = SerialMock::new($expectations);

    static mut WIFI_CMD_Q: Option<Queue<TestCommand, consts::U4, u8>> = None;
    static mut WIFI_RESP_Q: Option<Queue<Result<TestResponse, ATError>, consts::U4, u8>> = None;

    unsafe { WIFI_CMD_Q = Some(Queue::u8()) };
    unsafe { WIFI_RESP_Q = Some(Queue::u8()) };

    let (wifi_cmd_p, wifi_cmd_c) = unsafe { WIFI_CMD_Q.as_mut().unwrap().split() };
    let (wifi_resp_p, wifi_resp_c) = unsafe { WIFI_RESP_Q.as_mut().unwrap().split() };

    let test_at = ATParser::new(wifi, (wifi_cmd_c, wifi_resp_p));
    (test_at, (wifi_cmd_p, wifi_resp_c))
  }};
}

macro_rules! spin {
  ($at: expr, $len: expr) => {
    $at.spin();
    for _ in 0..$len {
      $at.handle_irq();
    }
    $at.spin();
  };
}

macro_rules! cleanup {
  ($at: expr, $wifi_resp_c: expr) => {
    let (mut serial, (mut wifi_cmd_c, _)) = $at.release();
    assert!($wifi_resp_c.dequeue().is_none());
    assert!(wifi_cmd_c.dequeue().is_none());
    serial.done();
  };
}

#[test]
fn test_at_command_echo() {
  let expected_response = b"AT\r\nOK\r\n";
  let expectations = [
    SerialTransaction::write_many(b"AT\r\n"),
    SerialTransaction::read_many(expected_response),
  ];

  let (mut test_at, (mut wifi_cmd_p, mut wifi_resp_c)) = setup!(&expectations);

  wifi_cmd_p.enqueue(TestCommand::AT).unwrap();

  spin!(test_at, expected_response.len());

  assert_eq!(
    wifi_resp_c.dequeue().unwrap().ok(),
    Some(TestResponse::None)
  );
  cleanup!(test_at, wifi_resp_c);
}

#[test]
fn test_at_command() {
  let expected_response = b"OK\r\n";
  let expectations = [
    SerialTransaction::write_many(b"AT\r\n"),
    SerialTransaction::read_many(expected_response),
  ];
  let (mut test_at, (mut wifi_cmd_p, mut wifi_resp_c)) = setup!(&expectations);

  wifi_cmd_p.enqueue(TestCommand::AT).unwrap();

  spin!(test_at, expected_response.len());

  assert_eq!(
    wifi_resp_c.dequeue().unwrap().ok(),
    Some(TestResponse::None)
  );
  cleanup!(test_at, wifi_resp_c);
}

#[test]
fn test_parameterized_command() {
  let expected_response = b"OK\r\n";
  let expectations = [
    SerialTransaction::write_many(b"AT+UDDRP=1,testString,2\r\n"),
    SerialTransaction::read_many(expected_response),
  ];
  let (mut test_at, (mut wifi_cmd_p, mut wifi_resp_c)) = setup!(&expectations);

  wifi_cmd_p
    .enqueue(TestCommand::SetDefaultPeer {
      peer_id: 1,
      url: String::from("testString"),
      connect_scheme: 2,
    })
    .unwrap();

  spin!(test_at, expected_response.len());

  assert_eq!(
    wifi_resp_c.dequeue().unwrap().ok(),
    Some(TestResponse::None)
  );
  cleanup!(test_at, wifi_resp_c);
}

#[test]
fn test_response() {
  let expected_response = b"abcdef012345\r\nOK\r\n";
  let expectations = [
    SerialTransaction::write_many(b"AT+CGSN\r\n"),
    SerialTransaction::read_many(expected_response),
  ];
  let (mut test_at, (mut wifi_cmd_p, mut wifi_resp_c)) = setup!(&expectations);

  wifi_cmd_p.enqueue(TestCommand::GetSerialNum).unwrap();

  spin!(test_at, expected_response.len());

  assert_eq!(
    wifi_resp_c.dequeue().unwrap().ok(),
    Some(TestResponse::SerialNum {
      serial: String::from("abcdef012345")
    })
  );
  cleanup!(test_at, wifi_resp_c);
}

#[test]
fn test_error() {
  let expected_response = b"ERROR\r\n";
  let expectations = [
    SerialTransaction::write_many(b"AT+UDDRP=1,testString,2\r\n"),
    SerialTransaction::read_many(expected_response),
  ];
  let (mut test_at, (mut wifi_cmd_p, mut wifi_resp_c)) = setup!(&expectations);

  wifi_cmd_p
    .enqueue(TestCommand::SetDefaultPeer {
      peer_id: 1,
      url: String::from("testString"),
      connect_scheme: 2,
    })
    .unwrap();

  spin!(test_at, expected_response.len());

  assert_eq!(wifi_resp_c.dequeue().unwrap(), Err(ATError::ParseString));
  cleanup!(test_at, wifi_resp_c);
}

#[test]
fn test_parameterized_single_response() {
  let expected_response = b"+UMSM:0\r\nOK\r\n";
  let expectations = [
    SerialTransaction::write_many(b"AT+UMSM?\r\n"),
    SerialTransaction::read_many(expected_response),
  ];
  let (mut test_at, (mut wifi_cmd_p, mut wifi_resp_c)) = setup!(&expectations);

  wifi_cmd_p.enqueue(TestCommand::GetUMSM).unwrap();

  spin!(test_at, expected_response.len());

  assert_eq!(
    wifi_resp_c.dequeue().unwrap().ok(),
    Some(TestResponse::UMSM { start_mode: 0 })
  );
  cleanup!(test_at, wifi_resp_c);
}

#[test]
fn test_parameterized_multi_response() {
  let expected_response = b"+CSGT:0,test\r\nOK\r\n";
  let expectations = [
    SerialTransaction::write_many(b"AT+CSGT?\r\n"),
    SerialTransaction::read_many(expected_response),
  ];
  let (mut test_at, (mut wifi_cmd_p, mut wifi_resp_c)) = setup!(&expectations);

  wifi_cmd_p.enqueue(TestCommand::GetCSGT).unwrap();

  spin!(test_at, expected_response.len());

  assert_eq!(
    wifi_resp_c.dequeue().unwrap().ok(),
    Some(TestResponse::CSGT {
      mode: 0,
      text: String::from("test")
    })
  );
  cleanup!(test_at, wifi_resp_c);
}

#[test]
fn test_response_unsolicited() {
  let expected_response = b"+CSGT:0,test\r\nOK\r\n";
  let expected_unsolicited = b"+UUDPD:0\r\n";
  let expectations = [
    SerialTransaction::write_many(b"AT+CSGT?\r\n"),
    SerialTransaction::read_many(expected_response),
    SerialTransaction::read_many(expected_unsolicited),
  ];
  let (mut test_at, (mut wifi_cmd_p, mut wifi_resp_c)) = setup!(&expectations);

  wifi_cmd_p.enqueue(TestCommand::GetCSGT).unwrap();

  spin!(test_at, expected_response.len());
  spin!(test_at, expected_unsolicited.len());

  assert_eq!(
    wifi_resp_c.dequeue().unwrap().ok(),
    Some(TestResponse::CSGT {
      mode: 0,
      text: String::from("test")
    })
  );
  assert_eq!(
    wifi_resp_c.dequeue().unwrap().ok(),
    Some(TestResponse::PeerDisconnected { peer_handle: 0 })
  );
  cleanup!(test_at, wifi_resp_c);
}
