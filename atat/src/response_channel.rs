use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};

use crate::Response;

pub type ResponseChannel<const INGRESS_BUF_SIZE: usize> =
    PubSubChannel<CriticalSectionRawMutex, Response<INGRESS_BUF_SIZE>, 1, 1, 1>;

pub type ResponsePublisher<'sub, const INGRESS_BUF_SIZE: usize> =
    Publisher<'sub, CriticalSectionRawMutex, Response<INGRESS_BUF_SIZE>, 1, 1, 1>;

pub type ResponseSubscription<'sub, const INGRESS_BUF_SIZE: usize> =
    Subscriber<'sub, CriticalSectionRawMutex, Response<INGRESS_BUF_SIZE>, 1, 1, 1>;
