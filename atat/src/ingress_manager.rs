use bbqueue::framed::FrameProducer;
use heapless::Vec;

use crate::digest::{DigestResult, Digester};
use crate::error::InternalError;
use crate::helpers::LossyStr;

pub struct IngressManager<
    D,
    const BUF_LEN: usize,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
> where
    D: Digester,
{
    /// Buffer holding incoming bytes.
    buf: Vec<u8, BUF_LEN>,

    /// The response producer sends responses to the client
    res_p: FrameProducer<'static, RES_CAPACITY>,
    /// The URC producer sends URCs to the client
    urc_p: FrameProducer<'static, URC_CAPACITY>,

    /// Digester.
    digester: D,
}

impl<D, const BUF_LEN: usize, const RES_CAPACITY: usize, const URC_CAPACITY: usize>
    IngressManager<D, BUF_LEN, RES_CAPACITY, URC_CAPACITY>
where
    D: Digester,
{
    pub fn new(
        res_p: FrameProducer<'static, RES_CAPACITY>,
        urc_p: FrameProducer<'static, URC_CAPACITY>,
        digester: D,
    ) -> Self {
        Self {
            buf: Vec::new(),
            res_p,
            urc_p,
            digester,
        }
    }

    fn enqueue_encoded_header<'a, const N: usize>(
        producer: &mut FrameProducer<'static, N>,
        header: impl Into<crate::error::Encoded<'a>>,
    ) -> Result<(), ()> {
        let header = header.into();
        if let Ok(mut grant) = producer.grant(header.len()) {
            match header {
                crate::error::Encoded::Simple(h) => grant[..1].copy_from_slice(&[h]),
                crate::error::Encoded::Nested(h, b) => {
                    grant[..1].copy_from_slice(&[h]);
                    grant[1..2].copy_from_slice(&[b]);
                }
                crate::error::Encoded::Array(h, b) => {
                    grant[..1].copy_from_slice(&[h]);
                    grant[1..header.len()].copy_from_slice(&b);
                }
                crate::error::Encoded::Slice(h, b) => {
                    grant[..1].copy_from_slice(&[h]);
                    grant[1..header.len()].copy_from_slice(b);
                }
            };
            grant.commit(header.len());
            Ok(())
        } else {
            Err(())
        }
    }

    /// Write data into the internal buffer raw bytes being the core type allows
    /// the ingress manager to be abstracted over the communication medium.
    ///
    /// This function should be called by the UART Rx, either in a receive
    /// interrupt, or a DMA interrupt, to move data from the peripheral into the
    /// ingress manager receive buffer.
    pub fn write(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        if self.buf.extend_from_slice(data).is_err() {
            error!("OVERFLOW DATA! Buffer: {:?}", LossyStr(&self.buf));
            if Self::enqueue_encoded_header(&mut self.res_p, Err(InternalError::Overflow)).is_err()
            {
                error!("Response queue full!");
            }
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

    pub fn digest(&mut self) {
        if let Ok(swallowed) = match self.digester.digest(&self.buf) {
            (DigestResult::None, swallowed) => Ok(swallowed),
            (DigestResult::Prompt(prompt), swallowed) => {
                match Self::enqueue_encoded_header(&mut self.res_p, prompt) {
                    Ok(_) => Ok(swallowed),
                    Err(_) => {
                        error!("Response queue full!");
                        Err(())
                    }
                }
            }
            (DigestResult::Urc(urc_line), swallowed) => {
                if let Ok(mut grant) = self.urc_p.grant(urc_line.len()) {
                    grant.copy_from_slice(urc_line);
                    grant.commit(urc_line.len());
                    Ok(swallowed)
                } else {
                    error!("URC queue full!");
                    Err(())
                }
            }
            (DigestResult::Response(resp), swallowed) => {
                #[cfg(any(feature = "defmt", feature = "log"))]
                match &resp {
                    Ok(r) => {
                        if r.is_empty() {
                            debug!("Received OK")
                        } else {
                            debug!("Received response: \"{:?}\"", LossyStr(r.as_ref()));
                        }
                    }
                    Err(e) => {
                        error!("Received error response {:?}", e);
                    }
                };

                match Self::enqueue_encoded_header(&mut self.res_p, resp) {
                    Ok(_) => Ok(swallowed),
                    Err(_) => {
                        error!("Response queue full!");
                        Err(())
                    }
                }
            }
        } {
            self.buf.rotate_left(swallowed);
            self.buf.truncate(self.buf.len() - swallowed);
            // if !self.buf.is_empty() {
            //     trace!("Buffer remainder: \"{:?}\"", LossyStr(&self.buf));
            // }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{digest::ParseError, error::Response, AtDigester, Parser};
    use bbqueue::BBBuffer;

    const TEST_RX_BUF_LEN: usize = 256;
    const TEST_URC_CAPACITY: usize = 10;
    const TEST_RES_CAPACITY: usize = 10;

    enum UrcTestParser {}

    impl Parser for UrcTestParser {
        fn parse<'a>(_buf: &'a [u8]) -> Result<(&'a [u8], usize), ParseError> {
            Err(ParseError::NoMatch)
        }
    }

    #[test]
    fn overflow() {
        static mut RES_Q: BBBuffer<TEST_RES_CAPACITY> = BBBuffer::new();
        let (res_p, mut res_c) = unsafe { RES_Q.try_split_framed().unwrap() };

        static mut URC_Q: BBBuffer<TEST_URC_CAPACITY> = BBBuffer::new();
        let (urc_p, _urc_c) = unsafe { URC_Q.try_split_framed().unwrap() };

        let mut ingress =
            IngressManager::<_, TEST_RX_BUF_LEN, TEST_RES_CAPACITY, TEST_URC_CAPACITY>::new(
                res_p,
                urc_p,
                AtDigester::<UrcTestParser>::new(),
            );

        ingress.write(b"+USORD: 3,266,\"");
        for _ in 0..266 {
            ingress.write(b"s");
        }
        ingress.write(b"\"\r\n");
        ingress.digest();
        let mut grant = res_c.read().unwrap();
        grant.auto_release(true);

        let res = match Response::from(grant.as_ref()) {
            Response::Result(r) => r,
            Response::Prompt(_) => Ok(&[][..]),
        };

        assert_eq!(res, Err(InternalError::Overflow));
    }
}
