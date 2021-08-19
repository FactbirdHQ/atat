//! Type definitions for the queues used in this crate.

use bbqueue::framed::{FrameConsumer, FrameProducer};
use heapless::spsc::{Consumer, Producer, Queue};

pub use crate::Command;

pub const COM_CAPACITY: usize = 3;

// Consumers
pub type ComConsumer = Consumer<'static, Command, { COM_CAPACITY + 1 }>;

// Producers
pub type ComProducer = Producer<'static, Command, { COM_CAPACITY + 1 }>;

// Queues
pub type ComQueue = Queue<Command, { COM_CAPACITY + 1 }>;

pub struct Queues<const RES_CAPACITY: usize, const URC_CAPACITY: usize> {
    pub res_queue: (
        FrameProducer<'static, RES_CAPACITY>,
        FrameConsumer<'static, RES_CAPACITY>,
    ),
    pub urc_queue: (
        FrameProducer<'static, URC_CAPACITY>,
        FrameConsumer<'static, URC_CAPACITY>,
    ),
    pub com_queue: (ComProducer, ComConsumer),
}
