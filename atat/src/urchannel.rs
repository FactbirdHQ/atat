use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::{DynSubscriber, PubSubChannel, Publisher};

use crate::AtatUrc;

pub type UrcPublisher<'sub, Urc, const CAPACITY: usize, const SUBSCRIBERS: usize> =
    Publisher<'sub, CriticalSectionRawMutex, <Urc as AtatUrc>::Response, CAPACITY, SUBSCRIBERS, 1>;
pub type UrcSubscription<'sub, Urc> = DynSubscriber<'sub, <Urc as AtatUrc>::Response>;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    MaximumSubscribersReached,
}

pub trait AtatUrcChannel<Urc: AtatUrc> {
    fn subscribe<'sub>(&'sub self) -> Result<UrcSubscription<'sub, Urc>, Error>;
}

pub struct UrcChannel<Urc: AtatUrc, const CAPACITY: usize, const SUBSCRIBERS: usize>(
    PubSubChannel<CriticalSectionRawMutex, Urc::Response, CAPACITY, SUBSCRIBERS, 1>,
);

impl<Urc: AtatUrc, const CAPACITY: usize, const SUBSCRIBERS: usize>
    UrcChannel<Urc, CAPACITY, SUBSCRIBERS>
{
    pub const fn new() -> Self {
        Self(PubSubChannel::new())
    }

    pub fn publisher(&self) -> UrcPublisher<Urc, CAPACITY, SUBSCRIBERS> {
        self.0.publisher().unwrap()
    }
}

impl<Urc: AtatUrc, const CAPACITY: usize, const SUBSCRIBERS: usize> AtatUrcChannel<Urc>
    for UrcChannel<Urc, CAPACITY, SUBSCRIBERS>
{
    fn subscribe<'sub>(&'sub self) -> Result<UrcSubscription<'sub, Urc>, Error> {
        self.0
            .dyn_subscriber()
            .map_err(|_| Error::MaximumSubscribersReached)
    }
}
