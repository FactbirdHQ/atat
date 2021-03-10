use heapless::{consts, ArrayLength, Vec};

use crate::error::InternalError;
use crate::queues::{ComConsumer, ResProducer, UrcItem, UrcProducer};
use crate::Command;
use crate::{
    digest::{DefaultDigester, DigestResult, Digester},
    urc_matcher::{DefaultUrcMatcher, UrcMatcher},
};

pub struct IngressManager<
    BufLen = consts::U256,
    D = DefaultDigester,
    U = DefaultUrcMatcher,
    UrcCapacity = consts::U10,
> where
    BufLen: ArrayLength<u8>,
    UrcCapacity: ArrayLength<UrcItem<BufLen>>,
    U: UrcMatcher,
    D: Digester,
{
    /// Buffer holding incoming bytes.
    buf: Vec<u8, BufLen>,

    /// The response producer sends responses to the client
    res_p: ResProducer<BufLen>,
    /// The URC producer sends URCs to the client
    urc_p: UrcProducer<BufLen, UrcCapacity>,
    /// The command consumer receives commands from the client
    com_c: ComConsumer,

    /// Digester.
    digester: D,

    /// URC matcher.
    urc_matcher: U,
}

impl<BufLen, UrcCapacity> IngressManager<BufLen, DefaultDigester, DefaultUrcMatcher, UrcCapacity>
where
    BufLen: ArrayLength<u8>,
    UrcCapacity: ArrayLength<UrcItem<BufLen>>,
{
    #[must_use]
    pub fn new(
        res_p: ResProducer<BufLen>,
        urc_p: UrcProducer<BufLen, UrcCapacity>,
        com_c: ComConsumer,
    ) -> Self {
        Self::with_customs(
            res_p,
            urc_p,
            com_c,
            DefaultUrcMatcher::default(),
            DefaultDigester::default(),
        )
    }
}

impl<BufLen, U, D, UrcCapacity> IngressManager<BufLen, D, U, UrcCapacity>
where
    D: Digester,
    U: UrcMatcher,
    BufLen: ArrayLength<u8>,
    UrcCapacity: ArrayLength<UrcItem<BufLen>>,
{
    pub fn with_customs(
        res_p: ResProducer<BufLen>,
        urc_p: UrcProducer<BufLen, UrcCapacity>,
        com_c: ComConsumer,
        urc_matcher: U,
        digester: D,
    ) -> Self {
        Self {
            buf: Vec::new(),
            res_p,
            urc_p,
            com_c,
            urc_matcher,
            digester,
        }
    }

    /// Write data into the internal buffer raw bytes being the core type allows
    /// the ingress manager to be abstracted over the communication medium.
    ///
    /// This function should be called by the UART Rx, either in a receive
    /// interrupt, or a DMA interrupt, to move data from the peripheral into the
    /// ingress manager receive buffer.
    pub fn write(&mut self, data: &[u8]) {
        defmt::trace!("Write: \"{}\"", data);

        if self.buf.extend_from_slice(data).is_err() {
            defmt::error!(
                "OVERFLOW DATA! Buffer: {}",
                core::convert::AsRef::<[u8]>::as_ref(&self.buf)
            );
            self.notify_response(Err(InternalError::Overflow));
        }
    }

    /// Return the current length of the internal buffer
    ///
    /// This can be useful for custom flowcontrol implementations
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Returns whether the internal buffer is empty
    ///
    /// This can be useful for custom flowcontrol implementations
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the capacity of the internal buffer
    ///
    /// This can be useful for custom flowcontrol implementations
    #[allow(clippy::unused_self)]
    pub fn capacity(&self) -> usize {
        BufLen::to_usize()
    }

    /// Notify the client that an appropriate response code, or error has been
    /// received
    fn notify_response(&mut self, resp: Result<Vec<u8, BufLen>, InternalError>) {
        match &resp {
            Ok(r) => {
                if r.is_empty() {
                    defmt::debug!("Received OK")
                } else {
                    defmt::debug!("Received response: \"{=[u8]:a}\"", &r);
                }
            }
            Err(_e) => defmt::error!("Received error response"),
        }
        if self.res_p.ready() {
            unsafe { self.res_p.enqueue_unchecked(resp) };
        } else {
            // FIXME: Handle queue not being ready
            defmt::error!("Response queue full!");
        }
    }

    /// Notify the client that an unsolicited response code (URC) has been
    /// received
    fn notify_urc(&mut self, resp: Vec<u8, BufLen>) {
        defmt::debug!("Received response: \"{=[u8]:a}\"", &resp);

        if self.urc_p.ready() {
            unsafe { self.urc_p.enqueue_unchecked(resp) };
        } else {
            // FIXME: Handle queue not being ready
            defmt::error!("URC queue full!");
        }
    }

    /// Handle receiving internal config commands from the client.
    fn handle_com(&mut self) {
        if let Some(com) = self.com_c.dequeue() {
            match com {
                Command::Reset => {
                    self.digester.reset();
                    self.buf.clear();
                    defmt::trace!("Cleared complete buffer");
                }
                Command::ForceReceiveState => self.digester.force_receive_state(),
            }
        }
    }

    pub fn digest(&mut self) {
        loop {
            // Handle commands every loop to catch timeouts asap
            self.handle_com();

            match self.digester.digest(&mut self.buf, &mut self.urc_matcher) {
                DigestResult::None => return,
                DigestResult::Urc(urc_line) => self.notify_urc(urc_line),
                DigestResult::Response(resp) => self.notify_response(resp),
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::queues::{ComQueue, ResQueue, UrcQueue};
    use heapless::{consts, spsc::Queue};

    type TestRxBufLen = consts::U256;
    type TestUrcCapacity = consts::U10;

    #[test]
    fn overflow() {
        static mut RES_Q: ResQueue<TestRxBufLen> = Queue(heapless::i::Queue::u8());
        let (res_p, mut res_c) = unsafe { RES_Q.split() };
        static mut URC_Q: UrcQueue<TestRxBufLen, TestUrcCapacity> = Queue(heapless::i::Queue::u8());
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };
        static mut COM_Q: ComQueue = Queue(heapless::i::Queue::u8());
        let (_com_p, com_c) = unsafe { COM_Q.split() };

        let mut ingress = IngressManager::with_customs(
            res_p,
            urc_p,
            com_c,
            DefaultUrcMatcher::default(),
            DefaultDigester::default(),
        );

        ingress.write(b"+USORD: 3,266,\"");
        for _ in 0..266 {
            ingress.write(b"s");
        }
        ingress.write(b"\"\r\n");
        ingress.digest();
        assert_eq!(res_c.dequeue().unwrap(), Err(InternalError::Overflow));
    }
}
