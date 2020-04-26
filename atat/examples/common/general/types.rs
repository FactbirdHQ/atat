//! Argument and parameter types used by General Commands and Responses
use atat_derive::AtatEnum;
use ufmt::derive::uDebug;

#[derive(uDebug, Clone, PartialEq, AtatEnum)]
#[at_enum(u8)]
pub enum Snt {
    /// (default value): International Mobile station Equipment Identity (IMEI)
    #[at_arg(value = 0)]
    IMEI,
    /// International Mobile station Equipment Identity and Software Version number(IMEISV)
    #[at_arg(value = 1)]
    IMEISV,
    /// Software Version Number (SVN)
    #[at_arg(value = 3)]
    SVN,
    /// IMEI (not including the spare digit), the check digit and the SVN
    #[at_arg(value = 255)]
    IMEIExtended,
}

/// Indicates the basic message indication type
#[derive(uDebug, Clone, PartialEq, AtatEnum)]
#[at_enum(u8)]
pub enum MessageIndicationType {
    /// • 1: Voice Message Waiting (third level method) or Voice Message Waiting on Line 1
    /// (CPHS method)
    #[at_arg(value = 1)]
    VoiceMessage,
    /// • 2: Fax Message Waiting
    #[at_arg(value = 2)]
    FaxMessage,
    /// • 3: Electronic Mail Message Waiting
    #[at_arg(value = 3)]
    EmailMessage,
    /// • 4: Extended Message Type Waiting (i.e. see the 3GPP TS 23.038 [7])
    #[at_arg(value = 4)]
    ExtendedMessage,
    /// • 5: Video Message Waiting
    #[at_arg(value = 5)]
    VideoMessage,
    /// • 6: Voice Message Waiting on Line 2 (CPHS method)
    #[at_arg(value = 6)]
    VoiceMessageLine2,
    /// • 7: reserved for future use
    #[at_arg(value = 7)]
    Reserved,
}
