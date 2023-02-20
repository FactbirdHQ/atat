use crate::{
    frame::{Frame, FrameProducerExt},
    helpers::LossyStr,
    DigestResult, Digester,
};
use bbqueue::framed::FrameProducer;
use embedded_io::asynch::Read;
use embedded_io::Error;
use heapless::Vec;

pub trait AtatIngress {
    fn write(&mut self, buffer: &[u8]);

    /// Read all bytes from the provided serial and ingest the read bytes into
    /// the ingress from where they will be processed
    async fn read_from(&mut self, serial: &mut impl Read) -> ! {
        loop {
            let mut buffer = [0; 32];
            match serial.read(&mut buffer).await {
                Ok(received) => {
                    if received > 0 {
                        self.write(&buffer[..received])
                    }
                }
                Err(e) => {
                    error!("Got serial read error {:?}", e.kind());
                }
            }
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
    fn write(&mut self, buffer: &[u8]) {
        if self.buffer.extend_from_slice(buffer).is_err() {
            error!("DATA OVERFLOW! Buffer: {:?}", LossyStr(&self.buffer));
            if self.res_writer.enqueue(Frame::OverflowError).is_err() {
                error!("Response queue full!");
            }
        }

        self.process()
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
    buffer: Vec<u8, INGRESS_BUF_SIZE>,
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
            buffer: Vec::new(),
            res_writer,
            urc_writer,
        }
    }

    /// Process all bytes currently in the ingress buffer
    fn process(&mut self) {
        trace!("Digesting: {:?}", LossyStr(&self.buffer));

        while !self.buffer.is_empty() {
            let swallowed = match self.digester.digest(&self.buffer) {
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

            self.buffer.rotate_left(used);
            self.buffer.truncate(self.buffer.len() - used);
        }
    }
}
