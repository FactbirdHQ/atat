/// Enumeration of message errors, as defined in 3GPP TS 27.005 version 10
/// section 3.2.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmsError {
    /// 3GPP TS 24.011 [6] clause E.2
    RelayProtocolCause(u16),
    /// 3GPP TS 23.040 [3] clause 9.2.3.22
    TransferProtocolFailureCause(u16),
    /// nick=MeFailure
    MeFailure,
    /// nick=SmsServiceReserved
    SmsServiceReserved,
    /// nick=NotAllowed
    NotAllowed,
    /// nick=NotSupported
    NotSupported,
    /// nick=InvalidPduParameter
    InvalidPduParameter,
    /// nick=InvalidTextParameter
    InvalidTextParameter,
    /// nick=SimNotInserted
    SimNotInserted,
    /// nick=SimPin
    SimPin,
    /// nick=PhSimPin
    PhSimPin,
    /// nick=SimFailure
    SimFailure,
    /// nick=SimBusy
    SimBusy,
    /// nick=SimWrong
    SimWrong,
    /// nick=SimPuk
    SimPuk,
    /// nick=SimPin2
    SimPin2,
    /// nick=SimPuk2
    SimPuk2,
    /// nick=MemoryFailure
    MemoryFailure,
    /// nick=InvalidIndex
    InvalidIndex,
    /// nick=MemoryFull
    MemoryFull,
    /// nick=SmscAddressUnknown
    SmscAddressUnknown,
    /// nick=NoNetwork
    NoNetwork,
    /// nick=NetworkTimeout
    NetworkTimeout,
    /// nick=NoCnmaAckExpected
    NoCnmaAckExpected,
    /// nick=Unknown
    Unknown,
    /// Other values in range `256..=511` are reserved.
    Reserved(u16),
    /// All values in range `512..` are manufacturer specific.
    ManufacturerSpecific(u16),
}

impl From<u16> for CmsError {
    fn from(v: u16) -> Self {
        match v {
            0..=127 => Self::RelayProtocolCause(v),
            128..=255 => Self::TransferProtocolFailureCause(v),
            300 => Self::MeFailure,
            301 => Self::SmsServiceReserved,
            302 => Self::NotAllowed,
            303 => Self::NotSupported,
            304 => Self::InvalidPduParameter,
            305 => Self::InvalidTextParameter,
            310 => Self::SimNotInserted,
            311 => Self::SimPin,
            312 => Self::PhSimPin,
            313 => Self::SimFailure,
            314 => Self::SimBusy,
            315 => Self::SimWrong,
            316 => Self::SimPuk,
            317 => Self::SimPin2,
            318 => Self::SimPuk2,
            320 => Self::MemoryFailure,
            321 => Self::InvalidIndex,
            322 => Self::MemoryFull,
            330 => Self::SmscAddressUnknown,
            331 => Self::NoNetwork,
            332 => Self::NetworkTimeout,
            340 => Self::NoCnmaAckExpected,
            500 => Self::Unknown,
            256..=511 => Self::Reserved(v),
            512.. => Self::ManufacturerSpecific(v),
        }
    }
}

impl From<CmsError> for u16 {
    fn from(error: CmsError) -> Self {
        match error {
            CmsError::MeFailure => 300,
            CmsError::SmsServiceReserved => 301,
            CmsError::NotAllowed => 302,
            CmsError::NotSupported => 303,
            CmsError::InvalidPduParameter => 304,
            CmsError::InvalidTextParameter => 305,
            CmsError::SimNotInserted => 310,
            CmsError::SimPin => 311,
            CmsError::PhSimPin => 312,
            CmsError::SimFailure => 313,
            CmsError::SimBusy => 314,
            CmsError::SimWrong => 315,
            CmsError::SimPuk => 316,
            CmsError::SimPin2 => 317,
            CmsError::SimPuk2 => 318,
            CmsError::MemoryFailure => 320,
            CmsError::InvalidIndex => 321,
            CmsError::MemoryFull => 322,
            CmsError::SmscAddressUnknown => 330,
            CmsError::NoNetwork => 331,
            CmsError::NetworkTimeout => 332,
            CmsError::NoCnmaAckExpected => 340,
            CmsError::Unknown => 500,
            CmsError::RelayProtocolCause(error)
            | CmsError::TransferProtocolFailureCause(error)
            | CmsError::Reserved(error)
            | CmsError::ManufacturerSpecific(error) => error,
        }
    }
}

#[cfg(feature = "string_errors")]
impl CmsError {
    pub const fn from_msg(s: &[u8]) -> Self {
        // FIXME:
        match s {
            b"ME failure" => Self::MeFailure,
            b"SMS service reserved" => Self::SmsServiceReserved,
            b"Operation not allowed" => Self::NotAllowed,
            b"Operation not supported" => Self::NotSupported,
            b"Invalid PDU mode parameter" => Self::InvalidPduParameter,
            b"Invalid text mode parameter" => Self::InvalidTextParameter,
            b"SIM not inserted" => Self::SimNotInserted,
            b"SIM PIN required" => Self::SimPin,
            b"SIM failure" => Self::SimFailure,
            b"SIM busy" => Self::SimBusy,
            b"SIM wrong" => Self::SimWrong,
            b"SIM PUK required" => Self::SimPuk,
            b"Memory failure" => Self::MemoryFailure,
            b"Invalid index" => Self::InvalidIndex,
            b"Memory full" => Self::MemoryFull,
            b"SMSC address unknown" => Self::SmscAddressUnknown,
            b"No network" => Self::NoNetwork,
            b"Network timeout" => Self::NetworkTimeout,
            _ => Self::Unknown,
        }
    }
}

impl core::fmt::Display for CmsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RelayProtocolCause(error) => write!(f, "Relay protocol error {error}"),
            Self::TransferProtocolFailureCause(error) => {
                write!(f, "Transfer protocol error {error}")
            }
            Self::MeFailure => write!(f, "ME failure"),
            Self::SmsServiceReserved => write!(f, "SMS service reserved"),
            Self::NotAllowed => write!(f, "Operation not allowed"),
            Self::NotSupported => write!(f, "Operation not supported"),
            Self::InvalidPduParameter => write!(f, "Invalid PDU mode parameter"),
            Self::InvalidTextParameter => write!(f, "Invalid text mode parameter"),
            Self::SimNotInserted => write!(f, "SIM not inserted"),
            Self::SimPin => write!(f, "SIM PIN required"),
            Self::PhSimPin => write!(f, "PH-SIM PIN required"),
            Self::SimFailure => write!(f, "SIM failure"),
            Self::SimBusy => write!(f, "SIM busy"),
            Self::SimWrong => write!(f, "SIM wrong"),
            Self::SimPuk => write!(f, "SIM PUK required"),
            Self::SimPin2 => write!(f, "SIM PIN2 required"),
            Self::SimPuk2 => write!(f, "SIM PUK2 required"),
            Self::MemoryFailure => write!(f, "Memory failure"),
            Self::InvalidIndex => write!(f, "Invalid index"),
            Self::MemoryFull => write!(f, "Memory full"),
            Self::SmscAddressUnknown => write!(f, "SMSC address unknown"),
            Self::NoNetwork => write!(f, "No network"),
            Self::NetworkTimeout => write!(f, "Network timeout"),
            Self::NoCnmaAckExpected => write!(f, "No CNMA acknowledgement expected"),
            Self::Unknown => write!(f, "Unknown"),
            Self::Reserved(error) => write!(f, "Unknown reserved error {error}"),
            Self::ManufacturerSpecific(error) => write!(f, "Manufacturer specific error {error}"),
        }
    }
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for CmsError {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::RelayProtocolCause(error) => defmt::write!(f, "Relay protocol error {}", error),
            Self::TransferProtocolFailureCause(error) => {
                defmt::write!(f, "Transfer protocol error {}", error)
            }
            Self::MeFailure => defmt::write!(f, "ME failure"),
            Self::SmsServiceReserved => defmt::write!(f, "SMS service reserved"),
            Self::NotAllowed => defmt::write!(f, "Operation not allowed"),
            Self::NotSupported => defmt::write!(f, "Operation not supported"),
            Self::InvalidPduParameter => defmt::write!(f, "Invalid PDU mode parameter"),
            Self::InvalidTextParameter => defmt::write!(f, "Invalid text mode parameter"),
            Self::SimNotInserted => defmt::write!(f, "SIM not inserted"),
            Self::SimPin => defmt::write!(f, "SIM PIN required"),
            Self::PhSimPin => defmt::write!(f, "PH-SIM PIN required"),
            Self::SimFailure => defmt::write!(f, "SIM failure"),
            Self::SimBusy => defmt::write!(f, "SIM busy"),
            Self::SimWrong => defmt::write!(f, "SIM wrong"),
            Self::SimPuk => defmt::write!(f, "SIM PUK required"),
            Self::SimPin2 => defmt::write!(f, "SIM PIN2 required"),
            Self::SimPuk2 => defmt::write!(f, "SIM PUK2 required"),
            Self::MemoryFailure => defmt::write!(f, "Memory failure"),
            Self::InvalidIndex => defmt::write!(f, "Invalid index"),
            Self::MemoryFull => defmt::write!(f, "Memory full"),
            Self::SmscAddressUnknown => defmt::write!(f, "SMSC address unknown"),
            Self::NoNetwork => defmt::write!(f, "No network"),
            Self::NetworkTimeout => defmt::write!(f, "Network timeout"),
            Self::NoCnmaAckExpected => defmt::write!(f, "No CNMA acknowledgement expected"),
            Self::Unknown => defmt::write!(f, "Unknown"),
            Self::Reserved(error) => defmt::write!(f, "Unknown reserved error {}", error),
            Self::ManufacturerSpecific(error) => {
                defmt::write!(f, "Manufacturer specific error {}", error)
            }
        }
    }
}
