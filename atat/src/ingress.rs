use crate::{
    frame::{Frame, FrameProducerExt},
    helpers::LossyStr,
    DigestResult, Digester,
};
use bbqueue::framed::FrameProducer;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    ResponseQueueFull,
    UrcQueueFull,
}

pub trait AtatIngress {
    /// Get the write buffer of the ingress
    ///
    /// Bytes written to the buffer must be committed by calling advance.
    fn write_buf(&mut self) -> &mut [u8];

    /// Commit written bytes to the ingress and make them visible to the digester.
    fn try_advance(&mut self, commit: usize) -> Result<(), Error>;

    /// Commit written bytes to the ingress and make them visible to the digester.
    #[cfg(feature = "async")]
    async fn advance(&mut self, commit: usize);

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
                        self.advance(received).await;
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

    fn try_advance(&mut self, commit: usize) -> Result<(), Error> {
        self.pos += commit;
        assert!(self.pos <= self.buf.len());

        while !self.buf.is_empty() {
            let swallowed = match self.digester.digest(&self.buf) {
                (DigestResult::None, used) => used,
                (DigestResult::Prompt(prompt), swallowed) => {
                    self.res_writer
                        .try_enqueue(Frame::Prompt(prompt))
                        .map_err(|_| Error::ResponseQueueFull)?;
                    debug!("Received prompt");
                    swallowed
                }
                (DigestResult::Urc(urc_line), swallowed) => {
                    let mut grant = self
                        .urc_writer
                        .grant(urc_line.len())
                        .map_err(|_| Error::UrcQueueFull)?;
                    debug!("Received URC: {:?}", LossyStr(urc_line));
                    grant.copy_from_slice(urc_line);
                    grant.commit(urc_line.len());
                    swallowed
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

                    self.res_writer
                        .try_enqueue(resp.into())
                        .map_err(|_| Error::ResponseQueueFull)?;
                    swallowed
                }
            };

            if swallowed == 0 {
                break;
            }

            self.buf.copy_within(swallowed..self.pos, 0);
            self.pos -= swallowed;
        }

        Ok(())
    }

    #[cfg(feature = "async")]
    async fn advance(&mut self, commit: usize) {
        self.pos += commit;
        assert!(self.pos <= self.buf.len());

        while !self.buf.is_empty() {
            let swallowed = match self.digester.digest(&self.buf) {
                (DigestResult::None, used) => used,
                (DigestResult::Prompt(prompt), swallowed) => {
                    self.res_writer.enqueue(Frame::Prompt(prompt)).await;
                    debug!("Received prompt");
                    swallowed
                }
                (DigestResult::Urc(urc_line), swallowed) => {
                    let mut grant = self.urc_writer.grant_async(urc_line.len()).await.unwrap();
                    debug!("Received URC: {:?}", LossyStr(urc_line));
                    grant.copy_from_slice(urc_line);
                    grant.commit(urc_line.len());
                    swallowed
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

                    self.res_writer.enqueue(resp.into()).await;
                    swallowed
                }
            };

            if swallowed == 0 {
                break;
            }

            self.buf.copy_within(swallowed..self.pos, 0);
            self.pos -= swallowed;
        }
    }
}
