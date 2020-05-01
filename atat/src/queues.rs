//! Type definitions for the queues used in this crate.

use heapless::spsc::{Consumer, Producer, Queue};
use heapless::{consts, Vec};

pub use crate::error::Error;
pub use crate::Command;

// Queue capacities
type ComCapacity = consts::U3;
type ResCapacity = consts::U5;
type UrcCapacity = consts::U10;

// Queue item types
type ComItem = Command;
type ResItem<BufLen> = Result<Vec<u8, BufLen>, Error>;
type UrcItem<BufLen> = Vec<u8, BufLen>;

// Note: We could create a simple macro to define producer, consumer and queue,
// but that would probably be harder to read than just the plain definitions.

// Consumers
pub(crate) type ComConsumer = Consumer<'static, ComItem, ComCapacity, u8>;
pub(crate) type ResConsumer<BufLen> = Consumer<'static, ResItem<BufLen>, ResCapacity, u8>;
pub(crate) type UrcConsumer<BufLen> = Consumer<'static, UrcItem<BufLen>, UrcCapacity, u8>;

// Producers
pub(crate) type ComProducer = Producer<'static, ComItem, ComCapacity, u8>;
pub(crate) type ResProducer<BufLen> = Producer<'static, ResItem<BufLen>, ResCapacity, u8>;
pub(crate) type UrcProducer<BufLen> = Producer<'static, UrcItem<BufLen>, UrcCapacity, u8>;

// Queues
pub type ComQueue = Queue<ComItem, ComCapacity, u8>;
pub type ResQueue<BufLen> = Queue<ResItem<BufLen>, ResCapacity, u8>;
pub type UrcQueue<BufLen> = Queue<UrcItem<BufLen>, UrcCapacity, u8>;
