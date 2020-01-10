extern crate env_logger;
extern crate std;

use super::*;
use core::fmt::Write;
use embedded_hal_mock::serial::{Mock as SerialMock, Transaction as SerialTransaction};

use heapless::{consts, spsc::Queue, String, Vec};
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
        env_logger::Builder::from_env(Env::default().default_filter_or("trace"))
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
enum TestResponseType {
    SingleSolicited(TestResponse),
    // MultiSolicited(Vec<TestResponse, heapless::consts::U4>),
    Unsolicited(TestUnsolicitedResponse),
    None,
}

#[derive(Debug, Clone, PartialEq)]
enum TestResponse {
    SerialNum {
        serial: String<MaxCommandLen>,
    },
    UMSM {
        start_mode: u8,
    },
    CSGT {
        mode: u8,
        text: String<MaxCommandLen>,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum TestUnsolicitedResponse {
    // Unsolicited responses
    PeerDisconnected { peer_handle: u8 },
}

impl ATRequestType for TestCommand {
    type Command = TestCommand;

    fn try_get_cmd(self) -> Option<Self::Command> {
        Some(self)
    }

    fn get_bytes<N: ArrayLength<u8>>(&self) -> Vec<u8, N> {
        let mut command = self.get_cmd();
        if !command.ends_with("\r\n") {
            command.push_str("\r\n").ok();
        }
        command.into_bytes()
    }
}

impl ATCommandInterface for TestCommand {
    type Response = TestResponseType;

    fn get_cmd<N: ArrayLength<u8>>(&self) -> String<N> {
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
    fn parse_resp(
        &self,
        response_lines: &mut Vec<String<MaxCommandLen>, MaxResponseLines>,
    ) -> TestResponseType {
        if response_lines.is_empty() {
            return TestResponseType::None;
        }
        let mut responses: Vec<Vec<&str, MaxResponseLines>, MaxResponseLines> =
            utils::split_parameterized_resp(response_lines);

        let response = responses.pop().unwrap();

        match *self {
            TestCommand::AT => TestResponseType::None,
            TestCommand::GetUMSM => TestResponseType::SingleSolicited(TestResponse::UMSM {
                start_mode: response[0].parse::<u8>().unwrap(),
            }),
            TestCommand::GetCSGT => TestResponseType::SingleSolicited(TestResponse::CSGT {
                mode: response[0].parse::<u8>().unwrap(),
                text: String::from(response[1]),
            }),
            TestCommand::GetSerialNum => {
                TestResponseType::SingleSolicited(TestResponse::SerialNum {
                    serial: String::from(response[0]),
                })
            }
            TestCommand::SetDefaultPeer { .. } => TestResponseType::None,
        }
    }

    fn parse_unsolicited(response_line: &str) -> Option<TestResponseType> {
        let (cmd, parameters) = utils::split_parameterized_unsolicited(response_line);

        Some(match cmd {
            "+UUDPD" => TestResponseType::Unsolicited(TestUnsolicitedResponse::PeerDisconnected {
                peer_handle: parameters[0].parse::<u8>().unwrap(),
            }),
            _ => return None,
        })
    }
}

#[derive(Clone, Copy)]
struct MilliSeconds(u32);
trait U32Ext {
    fn s(self) -> MilliSeconds;
    fn ms(self) -> MilliSeconds;
}
impl U32Ext for u32 {
    fn s(self) -> MilliSeconds {
        MilliSeconds(self / 1000)
    }
    fn ms(self) -> MilliSeconds {
        MilliSeconds(self)
    }
}

struct Timer6;
impl embedded_hal::timer::CountDown for Timer6 {
    type Time = MilliSeconds;
    fn start<T>(&mut self, _: T)
    where
        T: Into<MilliSeconds>,
    {
    }
    fn wait(&mut self) -> ::nb::Result<(), void::Void> {
        Ok(())
    }
}
impl embedded_hal::timer::Cancel for Timer6 {
    type Error = ();
    fn cancel(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

macro_rules! setup {
    ($expectations: expr) => {{
        setup_log();

        let serial = SerialMock::new($expectations);

        static mut REQ_Q: Option<Queue<TestCommand, consts::U10, u8>> = None;
        static mut RES_Q: Option<Queue<Result<TestResponseType, ATError>, consts::U10, u8>> = None;

        unsafe { REQ_Q = Some(Queue::u8()) };
        unsafe { RES_Q = Some(Queue::u8()) };

        let (req_p, req_c) = unsafe { REQ_Q.as_mut().unwrap().split() };
        let (res_p, res_c) = unsafe { RES_Q.as_mut().unwrap().split() };

        let at = client::ATClient::new((req_p, res_c), 1000.ms(), Timer6);

        let parser = ATParser::<_, _, consts::U100, _, _>::new(serial, (req_c, res_p));
        (parser, at.release())
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
    ($parser: expr, $res_c: expr) => {
        let (mut serial, (mut req_c, _)) = $parser.release();
        assert!($res_c.dequeue().is_none());
        assert!(req_c.dequeue().is_none());
        serial.done();
    };
}

#[test]
fn test_at_command_echo() {
    let expected_response = b"AT\r\nOK\r\n";
    let expectations = [
        SerialTransaction::flush(),
        SerialTransaction::read_many([0x00, 0xFF]),
        SerialTransaction::write_many(b"AT\r\n"),
        SerialTransaction::flush(),
        SerialTransaction::read_many(expected_response),
    ];

    let (mut parser, (mut req_p, mut res_c)) = setup!(&expectations);

    req_p.enqueue(TestCommand::AT).unwrap();

    spin!(parser, expected_response.len());

    assert_eq!(res_c.dequeue().unwrap().ok(), Some(TestResponseType::None));
    cleanup!(parser, res_c);
}

#[test]
fn test_at_command() {
    let expected_response = b"OK\r\n";
    let expectations = [
        SerialTransaction::flush(),
        SerialTransaction::read_many([0x00, 0xFF]),
        SerialTransaction::write_many(b"AT\r\n"),
        SerialTransaction::flush(),
        SerialTransaction::read_many(expected_response),
    ];
    let (mut parser, (mut req_p, mut res_c)) = setup!(&expectations);

    req_p.enqueue(TestCommand::AT).unwrap();

    spin!(parser, expected_response.len());

    assert_eq!(
        res_c.dequeue().unwrap().ok(),
        Some(TestResponseType::None)
    );
    cleanup!(parser, res_c);
}

#[test]
fn test_parameterized_command() {
    let expected_response = b"OK\r\n";
    let expectations = [
        SerialTransaction::flush(),
        SerialTransaction::read_many([0x00, 0xFF]),
        SerialTransaction::write_many(b"AT+UDDRP=1,testString,2\r\n"),
        SerialTransaction::flush(),
        SerialTransaction::read_many(expected_response),
    ];
    let (mut parser, (mut req_p, mut res_c)) = setup!(&expectations);

    req_p
        .enqueue(TestCommand::SetDefaultPeer {
            peer_id: 1,
            url: String::from("testString"),
            connect_scheme: 2,
        })
        .unwrap();

    spin!(parser, expected_response.len());

    assert_eq!(
        res_c.dequeue().unwrap().ok(),
        Some(TestResponseType::None)
    );
    cleanup!(parser, res_c);
}

#[test]
fn test_response() {
    let expected_response = b"abcdef012345\r\nOK\r\n";
    let expectations = [
        SerialTransaction::flush(),
        SerialTransaction::read_many([0x00, 0xFF]),
        SerialTransaction::write_many(b"AT+CGSN\r\n"),
        SerialTransaction::flush(),
        SerialTransaction::read_many(expected_response),
    ];
    let (mut parser, (mut req_p, mut res_c)) = setup!(&expectations);

    req_p.enqueue(TestCommand::GetSerialNum).unwrap();

    spin!(parser, expected_response.len());

    assert_eq!(
        res_c.dequeue().unwrap().ok(),
        Some(TestResponseType::SingleSolicited(TestResponse::SerialNum {
            serial: String::from("abcdef012345")
        }))
    );
    cleanup!(parser, res_c);
}

#[test]
fn test_error() {
    let expected_response = b"ERROR\r\n";
    let expectations = [
        SerialTransaction::flush(),
        SerialTransaction::read_many([0x00, 0xFF]),
        SerialTransaction::write_many(b"AT+UDDRP=1,testString,2\r\n"),
        SerialTransaction::flush(),
        SerialTransaction::read_many(expected_response),
    ];
    let (mut parser, (mut req_p, mut res_c)) = setup!(&expectations);

    req_p
        .enqueue(TestCommand::SetDefaultPeer {
            peer_id: 1,
            url: String::from("testString"),
            connect_scheme: 2,
        })
        .unwrap();

    spin!(parser, expected_response.len());

    assert_eq!(
        res_c.dequeue().unwrap(),
        Err(ATError::InvalidResponse)
    );
    cleanup!(parser, res_c);
}

#[test]
fn test_parameterized_single_response() {
    let expected_response = b"+UMSM:0\r\nOK\r\n";
    let expectations = [
        SerialTransaction::flush(),
        SerialTransaction::read_many([0x00, 0xFF]),
        SerialTransaction::write_many(b"AT+UMSM?\r\n"),
        SerialTransaction::flush(),
        SerialTransaction::read_many(expected_response),
    ];
    let (mut parser, (mut req_p, mut res_c)) = setup!(&expectations);

    req_p.enqueue(TestCommand::GetUMSM).unwrap();

    spin!(parser, expected_response.len());

    assert_eq!(
        res_c.dequeue().unwrap().ok(),
        Some(TestResponseType::SingleSolicited(TestResponse::UMSM {
            start_mode: 0
        }))
    );
    cleanup!(parser, res_c);
}

#[test]
fn test_parameterized_multi_response() {
    let expected_response = b"+CSGT:0,test\r\nOK\r\n";
    let expectations = [
        SerialTransaction::flush(),
        SerialTransaction::read_many([0x00, 0xFF]),
        SerialTransaction::write_many(b"AT+CSGT?\r\n"),
        SerialTransaction::flush(),
        SerialTransaction::read_many(expected_response),
    ];
    let (mut parser, (mut req_p, mut res_c)) = setup!(&expectations);

    req_p.enqueue(TestCommand::GetCSGT).unwrap();

    spin!(parser, expected_response.len());

    assert_eq!(
        res_c.dequeue().unwrap().ok(),
        Some(TestResponseType::SingleSolicited(TestResponse::CSGT {
            mode: 0,
            text: String::from("test")
        }))
    );
    cleanup!(parser, res_c);
}

#[test]
fn test_response_unsolicited() {
    let expected_response = b"+CSGT:0,test\r\nOK\r\n";
    let expected_unsolicited = b"+UUDPD:0\r\n";
    let expectations = [
        SerialTransaction::flush(),
        SerialTransaction::read_many([0x00, 0xFF]),
        SerialTransaction::write_many(b"AT+CSGT?\r\n"),
        SerialTransaction::flush(),
        SerialTransaction::read_many(expected_response),
        SerialTransaction::read_many(expected_unsolicited),
    ];
    let (mut parser, (mut req_p, mut res_c)) = setup!(&expectations);

    req_p.enqueue(TestCommand::GetCSGT).unwrap();

    spin!(parser, expected_response.len());
    spin!(parser, expected_unsolicited.len());

    assert_eq!(
        res_c.dequeue().unwrap().ok(),
        Some(TestResponseType::SingleSolicited(TestResponse::CSGT {
            mode: 0,
            text: String::from("test")
        }))
    );
    assert_eq!(
        res_c.dequeue().unwrap().ok(),
        Some(TestResponseType::Unsolicited(
            TestUnsolicitedResponse::PeerDisconnected { peer_handle: 0 }
        ))
    );
    cleanup!(parser, res_c);
}
