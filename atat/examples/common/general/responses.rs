//! Responses for General Commands
use atat::atat_derive::AtatResp;
use heapless::String;

/// 4.1 Manufacturer identification
/// Text string identifying the manufacturer.
#[derive(Clone, Debug, AtatResp)]
pub struct ManufacturerId {
    pub id: String<64>,
}

/// Model identification
/// Text string identifying the manufacturer.
#[derive(Clone, Debug, AtatResp)]
pub struct ModelId {
    pub id: String<64>,
}

/// Software version identification
/// Read a text string that identifies the software version of the module.
#[derive(Clone, Debug, AtatResp)]
pub struct SoftwareVersion {
    pub id: String<64>,
}

/// 7.11 Wi-Fi Access point station list +UWAPSTALIST
#[derive(Clone, AtatResp)]
pub struct WifiMac {
    pub mac_addr: heapless_bytes::Bytes<12>,
}
