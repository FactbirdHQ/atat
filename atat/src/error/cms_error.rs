/// Enumeration of message errors, as defined in 3GPP TS 27.005 version 10
/// section 3.2.5.
///
/// 0 -> 127 per 3GPP TS 24.011 [6] clause E.2 128 -> 255 per 3GPP TS 23.040 [3]
/// clause 9.2.3.22
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CmsError {
    /// nick=MeFailure
    MeFailure = 300,
    /// nick=SmsServiceReserved
    SmsServiceReserved = 301,
    /// nick=NotAllowed
    NotAllowed = 302,
    /// nick=NotSupported
    NotSupported = 303,
    /// nick=InvalidPduParameter
    InvalidPduParameter = 304,
    /// nick=InvalidTextParameter
    InvalidTextParameter = 305,
    /// nick=SimNotInserted
    SimNotInserted = 310,
    /// nick=SimPin
    SimPin = 311,
    /// nick=PhSimPin
    PhSimPin = 312,
    /// nick=SimFailure
    SimFailure = 313,
    /// nick=SimBusy
    SimBusy = 314,
    /// nick=SimWrong
    SimWrong = 315,
    /// nick=SimPuk
    SimPuk = 316,
    /// nick=SimPin2
    SimPin2 = 317,
    /// nick=SimPuk2
    SimPuk2 = 318,
    /// nick=MemoryFailure
    MemoryFailure = 320,
    /// nick=InvalidIndex
    InvalidIndex = 321,
    /// nick=MemoryFull
    MemoryFull = 322,
    /// nick=SmscAddressUnknown
    SmscAddressUnknown = 330,
    /// nick=NoNetwork
    NoNetwork = 331,
    /// nick=NetworkTimeout
    NetworkTimeout = 332,
    /// nick=NoCnmaAckExpected
    NoCnmaAckExpected = 340,
    /// nick=Unknown
    Unknown = 500,
}

impl From<u16> for CmsError {
    fn from(v: u16) -> Self {
        match v {
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
            _ => Self::Unknown,
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
        }
    }
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for CmsError {
    fn format(&self, f: defmt::Formatter) {
        match self {
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
        }
    }
}
