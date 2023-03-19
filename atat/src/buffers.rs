use bbqueue::BBBuffer;
use embedded_io::blocking::Write;

use crate::{urchannel::UrcChannel, AtatUrc, Config, Digester, Ingress};

/// Buffer size safety
///
/// BBQueue can only guarantee that issued write grants have half the size of its capacity.
/// In framed mode, each raw grant is prefixed with the size of the bbqueue frame.
/// We expect no larger frames than what can fit in a u16. For each [`crate::frame::Frame`] that is enqueued in the response queue,
/// a binconde dispatch byte is also appended (we use variable int encoding).
/// This means that to write an N byte response, we need a (3 + N) byte grant from the (non-framed) BBQueue.
///
/// The reason why this is behind the async feature flag is that it requires rust nightly.
/// Also, [`crate::AtatIngress.try_advance()`] (the non-async version) can return error if there is no room in the queues,
/// where the async equivalent simply returns () as it assumes that there at some point will be room in the queue.
///
/// One more additional note: We assume in the conditions that the digest result is never larger than the bytes that were input to the digester.
#[cfg(feature = "async")]
mod buf_safety {
    pub struct ConstCheck<const CHECK: bool>;

    const BBQUEUE_FRAME_HEADER_SIZE: usize = 2;
    const RES_FRAME_DISPATCH_SIZE: usize = 1;

    pub trait True {}
    impl True for ConstCheck<true> {}

    pub const fn is_valid_res_capacity<const INGRESS_BUF_SIZE: usize, const RES_CAPACITY: usize>(
    ) -> bool {
        RES_CAPACITY >= 2 * (BBQUEUE_FRAME_HEADER_SIZE + RES_FRAME_DISPATCH_SIZE + INGRESS_BUF_SIZE)
    }
}

pub struct Buffers<
    Urc: AtatUrc,
    const INGRESS_BUF_SIZE: usize,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
    const URC_SUBSCRIBERS: usize,
> {
    res_queue: BBBuffer<RES_CAPACITY>,
    /// The URC pub/sub channel
    pub urc_channel: UrcChannel<Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
}

#[cfg(feature = "async")]
impl<
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
        const URC_SUBSCRIBERS: usize,
    > Buffers<Urc, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY, URC_SUBSCRIBERS>
where
    buf_safety::ConstCheck<
        { buf_safety::is_valid_res_capacity::<INGRESS_BUF_SIZE, RES_CAPACITY>() },
    >: buf_safety::True,
{
    pub const fn new() -> Self {
        Self {
            res_queue: BBBuffer::new(),
            urc_channel: UrcChannel::new(),
        }
    }
}

#[cfg(not(feature = "async"))]
impl<
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
        const URC_SUBSCRIBERS: usize,
    > Buffers<Urc, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY, URC_SUBSCRIBERS>
{
    pub const fn new() -> Self {
        Self {
            res_queue: BBBuffer::new(),
            urc_channel: UrcChannel::new(),
        }
    }
}

impl<
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
        const URC_SUBSCRIBERS: usize,
    > Buffers<Urc, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY, URC_SUBSCRIBERS>
{
    #[cfg(feature = "async")]
    pub fn split<W: embedded_io::asynch::Write, D: Digester>(
        &self,
        writer: W,
        digester: D,
        config: Config,
    ) -> (
        Ingress<D, Urc, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY, URC_SUBSCRIBERS>,
        crate::asynch::Client<W, INGRESS_BUF_SIZE, RES_CAPACITY>,
    ) {
        let (res_writer, res_reader) = self.res_queue.try_split_framed().unwrap();

        (
            Ingress::new(digester, res_writer, self.urc_channel.publisher()),
            crate::asynch::Client::new(writer, res_reader, config),
        )
    }

    pub fn split_blocking<W: Write, D: Digester>(
        &self,
        writer: W,
        digester: D,
        config: Config,
    ) -> (
        Ingress<D, Urc, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY, URC_SUBSCRIBERS>,
        crate::blocking::Client<W, INGRESS_BUF_SIZE, RES_CAPACITY>,
    ) {
        let (res_writer, res_reader) = self.res_queue.try_split_framed().unwrap();

        (
            Ingress::new(digester, res_writer, self.urc_channel.publisher()),
            crate::blocking::Client::new(writer, res_reader, config),
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn show_why_we_need_two_times_bbqueue_capacity() {
        // If this test starts to fail in the future, then it may be because
        // bbqueue has relaxed its granting strategy, in which case the
        // buffer size safety checks should be revisited.

        let buffer = bbqueue::BBBuffer::<16>::new();
        let (mut producer, mut consumer) = buffer.try_split().unwrap();
        let grant = producer.grant_exact(9).unwrap();
        grant.commit(9);
        let grant = consumer.read().unwrap();
        grant.release(9);

        assert_eq!(
            Err(bbqueue::Error::InsufficientSize),
            producer.grant_exact(9)
        );
        assert!(producer.grant_exact(8).is_ok());
    }
}
