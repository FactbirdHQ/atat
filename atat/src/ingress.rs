use crate::{
    helpers::LossyStr, urc_channel::UrcPublisher, AtatUrc, DigestResult, Digester, ResponseSlot,
    UrcChannel,
};

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    ResponseSlotBusy,
    UrcChannelFull,
}

pub trait AtatIngress {
    /// Get the write buffer of the ingress
    ///
    /// Bytes written to the buffer must be committed by calling advance.
    fn write_buf(&mut self) -> &mut [u8];

    /// Commit a given number of written bytes to the ingress and make them visible to the digester.
    fn try_advance(&mut self, commit: usize) -> Result<(), Error>;

    /// Commit a given number of written bytes to the ingress and make them visible to the digester.
    #[cfg(feature = "async")]
    async fn advance(&mut self, commit: usize);

    /// Write a buffer to the ingress and return how many bytes were written.
    fn try_write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let mut buf = buf;
        let mut written = 0;
        while !buf.is_empty() {
            let ingress_buf = self.write_buf();
            if ingress_buf.is_empty() {
                return Ok(written);
            }
            let len = usize::min(buf.len(), ingress_buf.len());
            ingress_buf[..len].copy_from_slice(&buf[..len]);
            self.try_advance(len)?;
            buf = &buf[len..];
            written += len;
        }
        Ok(written)
    }

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
    async fn read_from(&mut self, serial: &mut impl embedded_io_async::Read) -> ! {
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
                    self.clear();
                }
            }
        }
    }

    fn clear(&mut self);
}

pub struct Ingress<
    'a,
    D: Digester,
    Urc: AtatUrc,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
    const URC_SUBSCRIBERS: usize,
> {
    digester: D,
    buf: [u8; INGRESS_BUF_SIZE],
    pos: usize,
    res_slot: &'a ResponseSlot<INGRESS_BUF_SIZE>,
    urc_publisher: UrcPublisher<'a, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
}

impl<
        'a,
        D: Digester,
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const URC_CAPACITY: usize,
        const URC_SUBSCRIBERS: usize,
    > Ingress<'a, D, Urc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>
{
    pub fn new(
        digester: D,
        res_slot: &'a ResponseSlot<INGRESS_BUF_SIZE>,
        urc_channel: &'a UrcChannel<Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
    ) -> Self {
        Self {
            digester,
            buf: [0; INGRESS_BUF_SIZE],
            pos: 0,
            res_slot,
            urc_publisher: urc_channel.0.publisher().unwrap(),
        }
    }
}

impl<
        D: Digester,
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const URC_CAPACITY: usize,
        const URC_SUBSCRIBERS: usize,
    > AtatIngress for Ingress<'_, D, Urc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>
{
    fn write_buf(&mut self) -> &mut [u8] {
        &mut self.buf[self.pos..]
    }

    fn try_advance(&mut self, commit: usize) -> Result<(), Error> {
        self.pos += commit;
        assert!(self.pos <= self.buf.len());

        while self.pos > 0 {
            let swallowed = match self.digester.digest(&self.buf[..self.pos]) {
                (DigestResult::None, swallowed) => {
                    if swallowed > 0 {
                        debug!(
                            "Received echo or space ({}/{}): {:?}",
                            swallowed,
                            self.pos,
                            LossyStr(&self.buf[..self.pos])
                        );
                    }

                    swallowed
                }
                (DigestResult::Prompt(prompt), swallowed) => {
                    debug!("Received prompt ({}/{})", swallowed, self.pos);

                    if self.res_slot.signal_prompt(prompt).is_err() {
                        error!("Received prompt but a response is already pending");
                    }

                    swallowed
                }
                (DigestResult::Urc(urc_line), swallowed) => {
                    if let Some(urc) = Urc::parse(urc_line) {
                        debug!(
                            "Received URC/{} ({}/{}): {:?}",
                            self.urc_publisher.space(),
                            swallowed,
                            self.pos,
                            LossyStr(urc_line)
                        );

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
                                debug!("Received OK ({}/{})", swallowed, self.pos,)
                            } else {
                                debug!(
                                    "Received response ({}/{}): {:?}",
                                    swallowed,
                                    self.pos,
                                    LossyStr(r)
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Received error response ({}/{}): {:?}",
                                swallowed, self.pos, e
                            );
                        }
                    }

                    if self.res_slot.signal_response(resp).is_err() {
                        error!("Received response but a response is already pending");
                    }
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
                (DigestResult::None, swallowed) => {
                    if swallowed > 0 {
                        debug!(
                            "Received echo or whitespace ({}/{}): {:?}",
                            swallowed,
                            self.pos,
                            LossyStr(&self.buf[..self.pos])
                        );
                    }

                    swallowed
                }
                (DigestResult::Prompt(prompt), swallowed) => {
                    debug!("Received prompt ({}/{})", swallowed, self.pos);

                    if self.res_slot.signal_prompt(prompt).is_err() {
                        error!("Received prompt but a response is already pending");
                    }
                    swallowed
                }
                (DigestResult::Urc(urc_line), swallowed) => {
                    if let Some(urc) = Urc::parse(urc_line) {
                        debug!(
                            "Received URC/{} ({}/{}): {:?}",
                            self.urc_publisher.space(),
                            swallowed,
                            self.pos,
                            LossyStr(urc_line)
                        );

                        if let Err(urc) = self.urc_publisher.try_publish(urc) {
                            self.urc_publisher.publish(urc).await;
                        }
                    } else {
                        error!("Parsing URC FAILED: {:?}", LossyStr(urc_line));
                    }
                    swallowed
                }
                (DigestResult::Response(resp), swallowed) => {
                    match &resp {
                        Ok(r) => {
                            if r.is_empty() {
                                debug!("Received OK ({}/{})", swallowed, self.pos,)
                            } else {
                                debug!(
                                    "Received response ({}/{}): {:?}",
                                    swallowed,
                                    self.pos,
                                    LossyStr(r)
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Received error response ({}/{}): {:?}",
                                swallowed, self.pos, e
                            );
                        }
                    }

                    if self.res_slot.signal_response(resp).is_err() {
                        error!("Received response but a response is already pending");
                    }
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

    fn clear(&mut self) {
        self.pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as atat, atat_derive::AtatUrc, response_slot::ResponseSlot, AtDigester, Response,
        UrcChannel,
    };

    use super::*;

    #[derive(AtatUrc, Clone, PartialEq, Debug)]
    enum Urc {
        #[at_urc(b"CONNECT OK")]
        ConnectOk,
        #[at_urc(b"CONNECT FAIL")]
        ConnectFail,
    }

    #[test]
    fn advance_can_processes_multiple_digest_results() {
        let res_slot = ResponseSlot::<100>::new();
        let urc_channel = UrcChannel::<Urc, 10, 1>::new();
        let mut ingress: Ingress<_, Urc, 100, 10, 1> =
            Ingress::new(AtDigester::<Urc>::new(), &res_slot, &urc_channel);

        let mut sub = urc_channel.subscribe().unwrap();

        let buf = ingress.write_buf();
        let data = b"\r\nCONNECT OK\r\n\r\nCONNECT FAIL\r\n\r\nOK\r\n";
        buf[..data.len()].copy_from_slice(data);
        ingress.try_advance(data.len()).unwrap();

        assert_eq!(Urc::ConnectOk, sub.try_next_message_pure().unwrap());
        assert_eq!(Urc::ConnectFail, sub.try_next_message_pure().unwrap());

        let response = res_slot.get();
        let response: &Response<100> = &response.borrow();
        assert_eq!(&Response::default(), response);
    }
}
