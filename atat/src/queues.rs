//! Type definitions for the queues used in this crate.

use heapless::{
    spsc::{Consumer, Producer, Queue},
    Vec,
};

pub use crate::error::InternalError;
pub use crate::Command;

// Queue item types
pub type ComItem = Command;
pub type ResItem<const BUF_LEN: usize> = Result<Vec<u8, BUF_LEN>, InternalError>;
pub type UrcItem<const BUF_LEN: usize> = Vec<u8, BUF_LEN>;

pub const RES_CAPACITY: usize = 1;
pub const COM_CAPACITY: usize = 3;

// Consumers
pub type ComConsumer = Consumer<'static, ComItem, { COM_CAPACITY + 1 }>;
pub type ResConsumer<const BUF_LEN: usize> = Consumer<'static, ResItem<BUF_LEN>, { RES_CAPACITY + 1 }>;
pub type UrcConsumer<const BUF_LEN: usize, const URC_CAPACITY: usize> =
    Consumer<'static, UrcItem<BUF_LEN>, URC_CAPACITY>;

// Producers
pub type ComProducer = Producer<'static, ComItem, { COM_CAPACITY + 1 }>;
pub type ResProducer<const BUF_LEN: usize> = Producer<'static, ResItem<BUF_LEN>, { RES_CAPACITY + 1 }>;
pub type UrcProducer<const BUF_LEN: usize, const URC_CAPACITY: usize> =
    Producer<'static, UrcItem<BUF_LEN>, URC_CAPACITY>;

// Queues
pub type ComQueue = Queue<ComItem, { COM_CAPACITY + 1 }>;
pub type ResQueue<const BUF_LEN: usize> = Queue<ResItem<BUF_LEN>, { RES_CAPACITY + 1 }>;
pub type UrcQueue<const BUF_LEN: usize, const URC_CAPACITY: usize> =
    Queue<UrcItem<BUF_LEN>, URC_CAPACITY>;

pub struct Queues<const BUF_LEN: usize, const URC_CAPACITY: usize> {
    pub res_queue: (ResProducer<BUF_LEN>, ResConsumer<BUF_LEN>),
    pub urc_queue: (
        UrcProducer<BUF_LEN, URC_CAPACITY>,
        UrcConsumer<BUF_LEN, URC_CAPACITY>,
    ),
    pub com_queue: (ComProducer, ComConsumer),
}
