use bbqueue::framed::FrameProducer;
use heapless::Vec;

use crate::Response;
use crate::atat_log;
use crate::error::InternalError;
use crate::helpers::LossyStr;
use crate::queues::ComConsumer;
use crate::Command;
use crate::{
    digest::{DefaultDigester, DigestResult, Digester},
    urc_matcher::{DefaultUrcMatcher, UrcMatcher},
};

pub struct IngressManager<
    D,
    U,
    const BUF_LEN: usize,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
> where
    U: UrcMatcher,
    D: Digester,
{
    /// Buffer holding incoming bytes.
    buf: Vec<u8, BUF_LEN>,

    /// The response producer sends responses to the client
    res_p: FrameProducer<'static, RES_CAPACITY>,
    /// The URC producer sends URCs to the client
    urc_p: FrameProducer<'static, URC_CAPACITY>,
    /// The command consumer receives commands from the client
    com_c: ComConsumer,

    /// Digester.
    digester: D,

    /// URC matcher.
    urc_matcher: U,
}

impl<const BUF_LEN: usize, const RES_CAPACITY: usize, const URC_CAPACITY: usize>
    IngressManager<DefaultDigester, DefaultUrcMatcher, BUF_LEN, RES_CAPACITY, URC_CAPACITY>
{
    #[must_use]
    pub fn new(
        res_p: FrameProducer<'static, RES_CAPACITY>,
        urc_p: FrameProducer<'static, URC_CAPACITY>,
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

impl<U, D, const BUF_LEN: usize, const RES_CAPACITY: usize, const URC_CAPACITY: usize>
    IngressManager<D, U, BUF_LEN, RES_CAPACITY, URC_CAPACITY>
where
    D: Digester,
    U: UrcMatcher,
{
    pub fn with_customs(
        res_p: FrameProducer<'static, RES_CAPACITY>,
        urc_p: FrameProducer<'static, URC_CAPACITY>,
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
            self.notify_response(Response::Error(InternalError::Overflow)).ok();
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
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Notify the client that an appropriate response code, or error has been
    /// received
    fn notify_response(&mut self, resp: Response<BUF_LEN>) -> Result<(), ()> {
        // #[cfg(any(feature = "defmt", feature = "log"))]
        // match &resp {
        //     Ok(r) => {
        //         if r.is_empty() {
        //             atat_log!(debug, "Received OK")
        //         } else {
        //             atat_log!(debug, "Received response: \"{:?}\"", LossyStr(r.as_ref()));
        //         }
        //     }
        //     Err(e) => {
        //         atat_log!(error, "Received error response {:?}", e);
        //     }
        // };

        let (header, bytes) = resp.as_bytes();
        if let Ok(mut grant) = self.res_p.grant(bytes.len() + header.len()) {
            grant[0..header.len()].copy_from_slice(&header);
            grant[header.len()..header.len() + bytes.len()].copy_from_slice(bytes);
            grant.commit(bytes.len() + header.len());
            Ok(())
        } else {
            atat_log!(error, "Response queue full!");
            Err(())
        }
    }

    /// Notify the client that an unsolicited response code (URC) has been
    /// received
    fn notify_urc(&mut self, resp: Vec<u8, BUF_LEN>) -> Result<(), ()> {
        atat_log!(debug, "Received response: \"{:?}\"", LossyStr(&resp));

        if let Ok(mut grant) = self.urc_p.grant(resp.len()) {
            grant.copy_from_slice(resp.as_ref());
            grant.commit(resp.len());
            Ok(())
        } else {
            atat_log!(error, "URC queue full!");
            Err(())
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
                DigestResult::None => Ok(()),
                DigestResult::Urc(urc_line) => self.notify_urc(urc_line),
                DigestResult::Response(resp) => self.notify_response(resp),
            }.ok();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::queues;
    use bbqueue::BBBuffer;
    use heapless::spsc::Queue;

    const TEST_RX_BUF_LEN: usize = 256;
    const TEST_URC_CAPACITY: usize = 10;
    const TEST_RES_CAPACITY: usize = 10;

    #[test]
    fn overflow() {
        static mut RES_Q: BBBuffer<TEST_RES_CAPACITY> = BBBuffer::new();
        let (res_p, mut res_c) = unsafe { RES_Q.try_split_framed().unwrap() };

        static mut URC_Q: BBBuffer<TEST_URC_CAPACITY> = BBBuffer::new();
        let (urc_p, _urc_c) = unsafe { URC_Q.try_split_framed().unwrap() };

        static mut COM_Q: queues::ComQueue = Queue::new();
        let (_com_p, com_c) = unsafe { COM_Q.split() };

        let mut ingress = IngressManager::<
            _,
            _,
            TEST_RX_BUF_LEN,
            TEST_RES_CAPACITY,
            TEST_URC_CAPACITY,
        >::with_customs(
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
        let mut grant = res_c.read().unwrap();
        grant.auto_release(true);
        assert_eq!(
            Response::from_bytes(grant.as_ref()),
            Response::<1>::Error(InternalError::Overflow)
        );
    }
}
