use crate::{
    frame::{Frame, FrameProducerExt},
    helpers::LossyStr,
    urchannel::UrcPublisher,
    AtatUrc, DigestResult, Digester,
};
use bbqueue::framed::FrameProducer;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    ResponseQueueFull,
    UrcChannelFull,
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

    /// Write a buffer to the ingress
    #[cfg(feature = "async")]
    async fn write(&mut self, buf: &[u8]) {
        let mut buf = buf;
        while !buf.is_empty() {
            let ingress_buf = self.write_buf();
            let len = usize::min(buf.len(), ingress_buf.len());
            ingress_buf[..len].copy_from_slice(&buf[..len]);
            self.advance(len).await;
            buf = &buf[len..];
        }
    }

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
    Urc: AtatUrc,
    const INGRESS_BUF_SIZE: usize,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
    const URC_SUBSCRIBERS: usize,
> {
    digester: D,
    buf: [u8; INGRESS_BUF_SIZE],
    pos: usize,
    res_writer: FrameProducer<'a, RES_CAPACITY>,
    urc_publisher: UrcPublisher<'a, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
}

impl<
        'a,
        D: Digester,
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
        const URC_SUBSCRIBERS: usize,
    > Ingress<'a, D, Urc, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY, URC_SUBSCRIBERS>
{
    pub(crate) fn new(
        digester: D,
        res_writer: FrameProducer<'a, RES_CAPACITY>,
        urc_publisher: UrcPublisher<'a, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
    ) -> Self {
        Self {
            digester,
            buf: [0; INGRESS_BUF_SIZE],
            pos: 0,
            res_writer,
            urc_publisher,
        }
    }
}

impl<
        D: Digester,
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
        const URC_SUBSCRIBERS: usize,
    > AtatIngress
    for Ingress<'_, D, Urc, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY, URC_SUBSCRIBERS>
{
    fn write_buf(&mut self) -> &mut [u8] {
        &mut self.buf[self.pos..]
    }

    fn try_advance(&mut self, commit: usize) -> Result<(), Error> {
        self.pos += commit;
        assert!(self.pos <= self.buf.len());

        while self.pos > 0 {
            let swallowed = match self.digester.digest(&self.buf[..self.pos]) {
                (DigestResult::None, used) => used,
                (DigestResult::Prompt(prompt), swallowed) => {
                    debug!("Received prompt");
                    self.res_writer
                        .try_enqueue(Frame::Prompt(prompt))
                        .map_err(|_| Error::ResponseQueueFull)?;
                    swallowed
                }
                (DigestResult::Urc(urc_line), swallowed) => {
                    if let Some(urc) = Urc::parse(urc_line) {
                        debug!("Received URC: {:?}", LossyStr(urc_line));
                        self.urc_publisher
                            .try_publish(urc)
                            .map_err(|_| Error::UrcChannelFull)?;
                    } else {
                        error!("Parsing URC FAILED: {:?}", LossyStr(urc_line));
                    }
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
                            warn!("Received error response {:?}", e);
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

        while self.pos > 0 {
            let swallowed = match self.digester.digest(&self.buf[..self.pos]) {
                (DigestResult::None, used) => used,
                (DigestResult::Prompt(prompt), swallowed) => {
                    debug!("Received prompt");
                    self.res_writer.enqueue(Frame::Prompt(prompt)).await;
                    swallowed
                }
                (DigestResult::Urc(urc_line), swallowed) => {
                    if let Some(urc) = Urc::parse(urc_line) {
                        debug!("Received URC: {:?}", LossyStr(urc_line));
                        self.urc_publisher.publish(urc).await;
                    } else {
                        error!("Parsing URC FAILED: {:?}", LossyStr(urc_line));
                    }
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
                            warn!("Received error response {:?}", e);
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
