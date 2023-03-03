use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::{DynSubscriber, PubSubChannel};

use crate::AtatUrc;

pub trait AtatUrcChannel<Urc: AtatUrc> {
    fn subscribe<'sub>(&'sub self) -> DynSubscriber<'sub, Urc::Response>;

    fn max_urc_len() -> usize;
}

pub struct UrcChannel<
    'a,
    Urc: AtatUrc,
    const INGRESS_BUF_SIZE: usize,
    const CAPACITY: usize,
    const SUBSCRIBERS: usize,
> {
    channel: &'a PubSubChannel<CriticalSectionRawMutex, Urc::Response, CAPACITY, SUBSCRIBERS, 1>,
}

impl<
        'a,
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const CAPACITY: usize,
        const SUBSCRIBERS: usize,
    > UrcChannel<'a, Urc, INGRESS_BUF_SIZE, CAPACITY, SUBSCRIBERS>
{
    pub(crate) fn new(
        channel: &'a PubSubChannel<
            CriticalSectionRawMutex,
            Urc::Response,
            CAPACITY,
            SUBSCRIBERS,
            1,
        >,
    ) -> Self {
        Self { channel }
    }
}

impl<
        'a,
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const CAPACITY: usize,
        const SUBSCRIBERS: usize,
    > AtatUrcChannel<Urc> for UrcChannel<'a, Urc, INGRESS_BUF_SIZE, CAPACITY, SUBSCRIBERS>
{
    fn subscribe<'sub>(&'sub self) -> DynSubscriber<'sub, Urc::Response> {
        self.channel.dyn_subscriber().unwrap()
    }

    fn max_urc_len() -> usize {
        INGRESS_BUF_SIZE
    }
}
