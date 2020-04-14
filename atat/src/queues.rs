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
type ResItem = Result<Vec<u8, consts::U256>, Error>;
type UrcItem = Vec<u8, consts::U256>;

// Note: We could create a simple macro to define producer, consumer and queue,
// but that would probably be harder to read than just the plain definitions.

// Consumers
pub(crate) type ComConsumer = Consumer<'static, ComItem, ComCapacity, u8>;
pub(crate) type ResConsumer = Consumer<'static, ResItem, ResCapacity, u8>;
pub(crate) type UrcConsumer = Consumer<'static, UrcItem, UrcCapacity, u8>;

// Producers
pub(crate) type ComProducer = Producer<'static, ComItem, ComCapacity, u8>;
pub(crate) type ResProducer = Producer<'static, ResItem, ResCapacity, u8>;
pub(crate) type UrcProducer = Producer<'static, UrcItem, UrcCapacity, u8>;

// Queues
pub(crate) type ComQueue = Queue<ComItem, ComCapacity, u8>;
pub(crate) type ResQueue = Queue<ResItem, ResCapacity, u8>;
pub(crate) type UrcQueue = Queue<UrcItem, UrcCapacity, u8>;
