use crate::{
    frame::{Frame, FrameProducerExt},
    helpers::LossyStr,
    DigestResult, Digester,
};
use bbqueue::framed::FrameProducer;
use heapless::Vec;

pub trait AtatIngress {
    /// Return the current length of the internal buffer
    ///
    /// This can be useful for custom flowcontrol implementations
    fn len(&self) -> usize;

    /// Returns whether the internal buffer is empty
    ///
    /// This can be useful for custom flowcontrol implementations
    fn is_empty(&self) -> bool;

    /// Return the capacity of the internal buffer
    ///
    /// This can be useful for custom flowcontrol implementations
    fn capacity(&self) -> usize;

    /// Write bytes to the ingress
    fn write(&mut self, buf: &[u8]);

    /// Read all bytes from the provided serial and ingest the read bytes into
    /// the ingress from where they will be processed
    #[cfg(feature = "async")]
    async fn read_from(&mut self, serial: &mut impl embedded_io::asynch::Read) -> ! {
        use embedded_io::Error;
        loop {
            let mut buf = [0; 32];
            match serial.read(&mut buf).await {
                Ok(received) => {
                    if received > 0 {
                        self.write(&buf[..received])
                    }
                }
                Err(e) => {
                    error!("Got serial read error {:?}", e.kind());
                }
            }
        }
    }
}

pub struct Ingress<
    'a,
    D: Digester,
    const INGRESS_BUF_SIZE: usize,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
> {
    digester: D,
    buf: Vec<u8, INGRESS_BUF_SIZE>,
    res_writer: FrameProducer<'a, RES_CAPACITY>,
    urc_writer: FrameProducer<'a, URC_CAPACITY>,
}

impl<
        'a,
        D: Digester,
        const INGRESS_BUF_SIZE: usize,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
    > Ingress<'a, D, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY>
{
    pub(crate) fn new(
        digester: D,
        res_writer: FrameProducer<'a, RES_CAPACITY>,
        urc_writer: FrameProducer<'a, URC_CAPACITY>,
    ) -> Self {
        Self {
            digester,
            buf: Vec::new(),
            res_writer,
            urc_writer,
        }
    }

    /// Process all bytes currently in the ingress buffer
    fn process(&mut self) {
        trace!("Digesting: {:?}", LossyStr(&self.buf));

        while !self.buf.is_empty() {
            let swallowed = match self.digester.digest(&self.buf) {
                (DigestResult::None, used) => Ok(used),
                (DigestResult::Prompt(prompt), swallowed) => {
                    if self.res_writer.enqueue(Frame::Prompt(prompt)).is_ok() {
                        debug!("Received prompt");
                        Ok(swallowed)
                    } else {
                        error!("Response queue full!");
                        Err(())
                    }
                }
                (DigestResult::Urc(urc_line), swallowed) => {
                    if let Ok(mut grant) = self.urc_writer.grant(urc_line.len()) {
                        debug!("Received URC: {:?}", LossyStr(urc_line));
                        grant.copy_from_slice(urc_line);
                        grant.commit(urc_line.len());
                        Ok(swallowed)
                    } else {
                        error!("URC queue full!");
                        Err(())
                    }
                }
                (DigestResult::Response(resp), swallowed) => {
                    match &resp {
                        Ok(r) => {
                            if r.is_empty() {
                                debug!("Received OK")
                            } else {
                                debug!("Received response: {:?}", LossyStr(r));
                            }
                        }
                        Err(e) => {
                            error!("Received error response {:?}", e);
                        }
                    }

                    if self.res_writer.enqueue(resp.into()).is_ok() {
                        Ok(swallowed)
                    } else {
                        error!("Response queue full!");
                        Err(())
                    }
                }
            };

            let used = swallowed.unwrap_or_default();
            if used == 0 {
                break;
            }

            self.buf.rotate_left(used);
            self.buf.truncate(self.buf.len() - used);
        }
    }
}

impl<
        D: Digester,
        const INGRESS_BUF_SIZE: usize,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
    > AtatIngress for Ingress<'_, D, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY>
{
    fn len(&self) -> usize {
        self.buf.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn capacity(&self) -> usize {
        self.buf.capacity()
    }
    
    fn write(&mut self, buf: &[u8]) {
        if self.buf.extend_from_slice(buf).is_err() {
            error!("DATA OVERFLOW! Buffer: {:?}", LossyStr(&self.buf));
            if self.res_writer.enqueue(Frame::OverflowError).is_err() {
                error!("Response queue full!");
            }
        }

        self.process()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{digest::ParseError, error::Response, AtDigester, InternalError, Parser};
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

        let frame = Frame::decode(grant.as_ref());
        let res = match Response::from(frame) {
            Response::Result(r) => r,
            Response::Prompt(_) => Ok(&[][..]),
        };

        assert_eq!(res, Err(InternalError::Overflow));
    }
}
