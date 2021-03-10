//! Type definitions for the queues used in this crate.

use heapless::Vec;
use heapless::{
    consts,
    spsc::{Consumer, Producer, Queue},
    ArrayLength,
};

pub use crate::error::InternalError;
pub use crate::Command;

// Queue item types
pub type ComItem = Command;
pub type ResItem<BufLen> = Result<Vec<u8, BufLen>, InternalError>;
pub type UrcItem<BufLen> = Vec<u8, BufLen>;

pub type ResCapacity = consts::U1;
pub type ComCapacity = consts::U3;

// Consumers
pub type ComConsumer = Consumer<'static, ComItem, ComCapacity, u8>;
pub type ResConsumer<BufLen> = Consumer<'static, ResItem<BufLen>, ResCapacity, u8>;
pub type UrcConsumer<BufLen, UrcCapacity> = Consumer<'static, UrcItem<BufLen>, UrcCapacity, u8>;

// Producers
pub type ComProducer = Producer<'static, ComItem, ComCapacity, u8>;
pub type ResProducer<BufLen> = Producer<'static, ResItem<BufLen>, ResCapacity, u8>;
pub type UrcProducer<BufLen, UrcCapacity> = Producer<'static, UrcItem<BufLen>, UrcCapacity, u8>;

// Queues
pub type ComQueue = Queue<ComItem, ComCapacity, u8>;
pub type ResQueue<BufLen> = Queue<ResItem<BufLen>, ResCapacity, u8>;
pub type UrcQueue<BufLen, UrcCapacity> = Queue<UrcItem<BufLen>, UrcCapacity, u8>;

pub struct Queues<BufLen, UrcCapacity>
where
    BufLen: ArrayLength<u8>,
    UrcCapacity: ArrayLength<UrcItem<BufLen>>,
{
    pub res_queue: (ResProducer<BufLen>, ResConsumer<BufLen>),
    pub urc_queue: (
        UrcProducer<BufLen, UrcCapacity>,
        UrcConsumer<BufLen, UrcCapacity>,
    ),
    pub com_queue: (ComProducer, ComConsumer),
}
