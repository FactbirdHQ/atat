use heapless::Vec;

use crate::atat_log;
use crate::error::InternalError;
use crate::helpers::LossyStr;
use crate::queues::{ComConsumer, ResProducer, UrcProducer};
use crate::Command;
use crate::{
    digest::{DefaultDigester, DigestResult, Digester},
    urc_matcher::{DefaultUrcMatcher, UrcMatcher},
};

pub struct IngressManager<D, U, const BUF_LEN: usize, const URC_CAPACITY: usize>
where
    U: UrcMatcher,
    D: Digester,
{
    /// Buffer holding incoming bytes.
    buf: Vec<u8, BUF_LEN>,

    /// The response producer sends responses to the client
    res_p: ResProducer<BUF_LEN>,
    /// The URC producer sends URCs to the client
    urc_p: UrcProducer<BUF_LEN, URC_CAPACITY>,
    /// The command consumer receives commands from the client
    com_c: ComConsumer,

    /// Digester.
    digester: D,

    /// URC matcher.
    urc_matcher: U,
}

impl<const BUF_LEN: usize, const URC_CAPACITY: usize>
    IngressManager<DefaultDigester, DefaultUrcMatcher, BUF_LEN, URC_CAPACITY>
{
    #[must_use]
    pub fn new(
        res_p: ResProducer<BUF_LEN>,
        urc_p: UrcProducer<BUF_LEN, URC_CAPACITY>,
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

impl<U, D, const BUF_LEN: usize, const URC_CAPACITY: usize>
    IngressManager<D, U, BUF_LEN, URC_CAPACITY>
where
    D: Digester,
    U: UrcMatcher,
{
    pub fn with_customs(
        res_p: ResProducer<BUF_LEN>,
        urc_p: UrcProducer<BUF_LEN, URC_CAPACITY>,
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
        atat_log!(trace, "Write: \"{:?}\"", LossyStr(data));

        if self.buf.extend_from_slice(data).is_err() {
            atat_log!(error, "OVERFLOW DATA! Buffer: {:?}", LossyStr(&self.buf));
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
        BUF_LEN
    }

    /// Notify the client that an appropriate response code, or error has been
    /// received
    fn notify_response(&mut self, resp: Result<Vec<u8, BUF_LEN>, InternalError>) {
        match &resp {
            Ok(r) => {
                if r.is_empty() {
                    atat_log!(debug, "Received OK")
                } else {
                    atat_log!(debug, "Received response: \"{:?}\"", LossyStr(r));
                }
            }
            Err(e) => atat_log!(error, "Received error response {:?}", e),
        }
        if self.res_p.ready() {
            unsafe { self.res_p.enqueue_unchecked(resp) };
        } else {
            // FIXME: Handle queue not being ready
            atat_log!(error, "Response queue full!");
        }
    }

    /// Notify the client that an unsolicited response code (URC) has been
    /// received
    fn notify_urc(&mut self, resp: Vec<u8, BUF_LEN>) {
        atat_log!(debug, "Received response: \"{:?}\"", LossyStr(&resp));

        if self.urc_p.ready() {
            unsafe { self.urc_p.enqueue_unchecked(resp) };
        } else {
            // FIXME: Handle queue not being ready
            atat_log!(error, "URC queue full!");
        }
    }

    /// Handle receiving internal config commands from the client.
    fn handle_com(&mut self) {
        if let Some(com) = self.com_c.dequeue() {
            match com {
                Command::Reset => {
                    atat_log!(
                        debug,
                        "Cleared complete buffer as requested by client [{:?}]",
                        LossyStr(&self.buf)
                    );
                    self.digester.reset();
                    self.buf.clear();
                }
                Command::ForceReceiveState => self.digester.force_receive_state(),
            }
        }
    }

    pub fn digest(&mut self) {
        for _ in 0..5 {
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
    use heapless::spsc::Queue;

    const TEST_RX_BUF_LEN: usize = 256;
    const TEST_URC_CAPACITY: usize = 10;

    #[test]
    fn overflow() {
        static mut RES_Q: ResQueue<TEST_RX_BUF_LEN> = Queue::new();
        let (res_p, mut res_c) = unsafe { RES_Q.split() };
        static mut URC_Q: UrcQueue<TEST_RX_BUF_LEN, TEST_URC_CAPACITY> = Queue::new();
        let (urc_p, _urc_c) = unsafe { URC_Q.split() };
        static mut COM_Q: ComQueue = Queue::new();
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
