//! ### 4 - General Commands
pub mod responses;
pub mod types;
pub mod urc;

use atat::{atat_derive::AtatCmd, AtatCmd, Error};
use heapless::{consts, String};
use responses::*;
use types::*;

/// 4.1 Manufacturer identification +CGMI
///
/// Text string identifying the manufacturer.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGMI", ManufacturerId)]
pub struct GetManufacturerId;

/// 4.7 IMEI identification +CGSN
///
/// Returns the product serial number, the International Mobile Equipment Identity (IMEI) of the MT.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGSN", IMEI)]
pub struct GetIMEI {
    #[at_arg(position = 0)]
    pub snt: Option<Snt>,
}

/// 4.12 Card identification +CCID
///
/// Returns the ICCID (Integrated Circuit Card ID) of the SIM-card. ICCID is a serial number identifying the SIM.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CCID", CCID)]
pub struct GetCCID;
