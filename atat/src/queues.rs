//! Type definitions for the queues used in this crate.

use heapless::spsc::{Consumer, Producer, Queue};
use heapless::Vec;

pub use crate::error::Error;
pub use crate::Command;

// Queue item types
pub(crate) type ComItem = Command;
pub(crate) type ResItem<BufLen> = Result<Vec<u8, BufLen>, Error>;
pub(crate) type UrcItem<BufLen> = Vec<u8, BufLen>;

// Note: We could create a simple macro to define producer, consumer and queue,
// but that would probably be harder to read than just the plain definitions.

// Consumers
pub(crate) type ComConsumer<ComCapacity> = Consumer<'static, ComItem, ComCapacity, u8>;
pub(crate) type ResConsumer<BufLen, ResCapacity> =
    Consumer<'static, ResItem<BufLen>, ResCapacity, u8>;
pub(crate) type UrcConsumer<BufLen, UrcCapacity> =
    Consumer<'static, UrcItem<BufLen>, UrcCapacity, u8>;

// Producers
pub(crate) type ComProducer<ComCapacity> = Producer<'static, ComItem, ComCapacity, u8>;
pub(crate) type ResProducer<BufLen, ResCapacity> =
    Producer<'static, ResItem<BufLen>, ResCapacity, u8>;
pub(crate) type UrcProducer<BufLen, UrcCapacity> =
    Producer<'static, UrcItem<BufLen>, UrcCapacity, u8>;

// Queues
pub type ComQueue<ComCapacity> = Queue<ComItem, ComCapacity, u8>;
pub type ResQueue<BufLen, ResCapacity> = Queue<ResItem<BufLen>, ResCapacity, u8>;
pub type UrcQueue<BufLen, UrcCapacity> = Queue<UrcItem<BufLen>, UrcCapacity, u8>;
