use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher},
};
use heapless::Vec;

pub type ResChannel<const INGRESS_BUF_SIZE: usize> =
    PubSubChannel<CriticalSectionRawMutex, Vec<u8, INGRESS_BUF_SIZE>, 1, 1, 1>;

pub type ResPublisher<'sub, const INGRESS_BUF_SIZE: usize> =
    Publisher<'sub, CriticalSectionRawMutex, Vec<u8, INGRESS_BUF_SIZE>, 1, 1, 1>;
