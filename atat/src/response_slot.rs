use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

use crate::Response;

pub type ResponseSlot<const INGRESS_BUF_SIZE: usize> =
    Signal<CriticalSectionRawMutex, Response<INGRESS_BUF_SIZE>>;
