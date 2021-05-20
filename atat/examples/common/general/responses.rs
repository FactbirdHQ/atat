//! Responses for General Commands
use atat::atat_derive::AtatResp;
use heapless::String;

/// 4.1 Manufacturer identification
/// Text string identifying the manufacturer.
#[derive(Clone, Debug, AtatResp)]
pub struct ManufacturerId {
    #[at_arg(position = 0)]
    pub id: String<64>,
}

/// 4.7 IMEI identification +CGSN
/// Returns the product serial number, the International Mobile Equipment Identity (IMEI) of the MT.
#[derive(Clone, Debug, AtatResp)]
pub struct IMEI {
    #[at_arg(position = 0)]
    pub imei: u64,
}

/// 4.12 Card identification +CCID
/// Returns the ICCID (Integrated Circuit Card ID) of the SIM-card. ICCID is a serial number identifying the SIM.
#[derive(Clone, Debug, AtatResp)]
pub struct CCID {
    #[at_arg(position = 0)]
    pub ccid: u128,
}
