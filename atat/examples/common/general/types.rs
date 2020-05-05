//! Argument and parameter types used by General Commands and Responses

use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Snt {
    /// (default value): International Mobile station Equipment Identity (IMEI)
    IMEI = 0,
    /// International Mobile station Equipment Identity and Software Version number(IMEISV)
    IMEISV = 2,
    /// Software Version Number (SVN)
    SVN = 3,
    /// IMEI (not including the spare digit), the check digit and the SVN
    IMEIExtended = 255,
}

/// Indicates the basic message indication type
#[derive(Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum MessageIndicationType {
    /// • 1: Voice Message Waiting (third level method) or Voice Message Waiting on Line 1
    /// (CPHS method)
    VoiceMessage = 1,
    /// • 2: Fax Message Waiting
    FaxMessage = 2,
    /// • 3: Electronic Mail Message Waiting
    EmailMessage = 3,
    /// • 4: Extended Message Type Waiting (i.e. see the 3GPP TS 23.038 [7])
    ExtendedMessage = 4,
    /// • 5: Video Message Waiting
    VideoMessage = 5,
    /// • 6: Voice Message Waiting on Line 2 (CPHS method)
    VoiceMessageLine2 = 6,
    /// • 7: reserved for future use
    Reserved = 7,
}
