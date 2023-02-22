use crate::{
    frame::{Frame, FrameProducerExt},
    helpers::LossyStr,
    DigestResult, Digester,
};
use bbqueue::framed::FrameProducer;

pub trait AtatIngress {
    /// Get the write buffer of the ingress
    /// 
    /// Bytes written to the buffer must be committed by calling advance.
    fn write_buf(&mut self) -> &mut [u8];

    /// Commit written bytes to the ingress and make them visible to the digester.
    fn advance(&mut self, commit: usize);

    /// Read all bytes from the provided serial and ingest the read bytes into
    /// the ingress from where they will be processed
    #[cfg(feature = "async")]
    async fn read_from(&mut self, serial: &mut impl embedded_io::asynch::Read) -> ! {
        use embedded_io::Error;
        loop {
            let buf = self.write_buf();
            match serial.read(buf).await {
                Ok(received) => {
                    if received > 0 {
                        self.advance(received);
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
    buf: [u8; INGRESS_BUF_SIZE],
    pos: usize,
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
            buf: [0; INGRESS_BUF_SIZE],
            pos: 0,
            res_writer,
            urc_writer,
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
    fn write_buf(&mut self) -> &mut [u8] {
        &mut self.buf[self.pos..]
    }

    fn advance(&mut self, commit: usize) {
        self.pos += commit;
        assert!(self.pos <= self.buf.len());

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

            self.buf.copy_within(used..self.pos, 0);
            self.pos -= used;
        }
    }
}
