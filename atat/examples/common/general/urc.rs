//! Responses for Internet protocol transport layer Commands
use super::types;
use atat::atat_derive::{AtatResp, AtatUrc};
use atat::{AtatResp, AtatUrc};

/// 11.29 Message waiting indication +UMWI
/// Provides information regarding the Message Waiting Indication (MWI) third level method (3GPP defined in
/// 3GPP TS 23.040 [8]) and CPHS method [53] following AT&T Device Requirements [48].
/// The set command enables / disables the URC presentation. The URCs are by default enabled.
/// MWI is based on specific EFs not present in all SIM cards. In case these EFs are not present, the information
/// text response is an error result code ("+CME ERROR: operation not allowed" if +CMEE is set to 2) and no URCs
/// will be displayed.
#[derive(Clone, AtatResp)]
pub struct MessageWaitingIndication {
    #[at_arg(position = 0)]
    pub status: u8,
    #[at_arg(position = 1)]
    pub code: types::MessageIndicationType,
}
