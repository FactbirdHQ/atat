use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::{PubSubChannel, Publisher, Subscriber};

use crate::AtatUrc;

pub type UrcPublisher<'sub, Urc, const CAPACITY: usize, const SUBSCRIBERS: usize> =
    Publisher<'sub, CriticalSectionRawMutex, <Urc as AtatUrc>::Response, CAPACITY, SUBSCRIBERS, 1>;
pub type UrcSubscription<'sub, Urc, const CAPACITY: usize, const SUBSCRIBERS: usize> =
    Subscriber<'sub, CriticalSectionRawMutex, <Urc as AtatUrc>::Response, CAPACITY, SUBSCRIBERS, 1>;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    MaximumSubscribersReached,
}

pub struct UrcChannel<Urc: AtatUrc, const CAPACITY: usize, const SUBSCRIBERS: usize>(
    pub(crate) PubSubChannel<CriticalSectionRawMutex, Urc::Response, CAPACITY, SUBSCRIBERS, 1>,
);

impl<Urc: AtatUrc, const CAPACITY: usize, const SUBSCRIBERS: usize>
    UrcChannel<Urc, CAPACITY, SUBSCRIBERS>
{
    pub const fn new() -> Self {
        Self(PubSubChannel::new())
    }

    pub fn subscribe(&self) -> Result<UrcSubscription<'_, Urc, CAPACITY, SUBSCRIBERS>, Error> {
        self.0
            .subscriber()
            .map_err(|_| Error::MaximumSubscribersReached)
    }

    pub fn free_capacity(&self) -> usize {
        self.0.free_capacity()
    }
}
