#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionError {
    Unknown = 0,
    NoCarrier = 1,
    NoDialtone = 2,
    Busy = 3,
    NoAnswer = 4,
}

impl From<u8> for ConnectionError {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::NoCarrier,
            2 => Self::NoDialtone,
            3 => Self::Busy,
            4 => Self::NoAnswer,
            _ => Self::Unknown,
        }
    }
}

impl core::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::NoCarrier => write!(f, "No carrier"),
            Self::NoDialtone => write!(f, "No dialtone"),
            Self::Busy => write!(f, "Busy"),
            Self::NoAnswer => write!(f, "No answer"),
        }
    }
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for ConnectionError {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Unknown => defmt::write!(f, "Unknown"),
            Self::NoCarrier => defmt::write!(f, "No carrier"),
            Self::NoDialtone => defmt::write!(f, "No dialtone"),
            Self::Busy => defmt::write!(f, "Busy"),
            Self::NoAnswer => defmt::write!(f, "No answer"),
        }
    }
}
