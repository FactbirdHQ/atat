//! Type definitions for the queues used in this crate.

use bbqueue::framed::{FrameConsumer, FrameProducer};

pub struct Queues<const RES_CAPACITY: usize, const URC_CAPACITY: usize> {
    pub res_queue: (
        FrameProducer<'static, RES_CAPACITY>,
        FrameConsumer<'static, RES_CAPACITY>,
    ),
    pub urc_queue: (
        FrameProducer<'static, URC_CAPACITY>,
        FrameConsumer<'static, URC_CAPACITY>,
    ),
}
