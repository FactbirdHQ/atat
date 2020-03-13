pub mod general;

use atat::{atat_derive::AtatUrc, AtatUrc};

use atat::{
    atat_derive::{AtatCmd, AtatResp},
    AtatCmd, AtatResp,
};
use heapless::String;

#[derive(Clone, AtatResp)]
pub struct NoResponse;

#[derive(Clone, AtatCmd)]
#[at_cmd("", NoResponse, timeout_ms = 1000)]
pub struct AT;

#[derive(Clone, AtatUrc)]
pub enum Urc {
    #[at_urc("+UMWI")]
    MessageWaitingIndication(general::urc::MessageWaitingIndication),
}
