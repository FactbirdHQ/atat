pub mod general;

use atat::{atat_derive::ATATUrc, ATATUrc};

use atat::{
    atat_derive::{ATATCmd, ATATResp},
    ATATCmd, ATATResp,
};
use heapless::String;

#[derive(Clone, ATATResp)]
pub struct NoResponse;

#[derive(Clone, ATATCmd)]
#[at_cmd("", NoResponse, timeout_ms = 1000)]
pub struct AT;

#[derive(Clone, ATATUrc)]
pub enum Urc {
    #[at_urc("+UMWI")]
    MessageWaitingIndication(general::urc::MessageWaitingIndication),
}
