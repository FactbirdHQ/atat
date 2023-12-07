use embedded_io::Write;

use crate::{
    response_channel::ResponseChannel, urc_channel::UrcChannel, AtatUrc, Config, Digester, Ingress,
};

pub struct Buffers<
    Urc: AtatUrc,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
    const URC_SUBSCRIBERS: usize,
> {
    res_channel: ResponseChannel<INGRESS_BUF_SIZE>,
    /// The URC pub/sub channel
    pub urc_channel: UrcChannel<Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
}

impl<
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const URC_CAPACITY: usize,
        const URC_SUBSCRIBERS: usize,
    > Buffers<Urc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>
{
    pub const fn new() -> Self {
        Self {
            res_channel: ResponseChannel::new(),
            urc_channel: UrcChannel::new(),
        }
    }
}

impl<
        Urc: AtatUrc,
        const INGRESS_BUF_SIZE: usize,
        const URC_CAPACITY: usize,
        const URC_SUBSCRIBERS: usize,
    > Buffers<Urc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>
{
    #[cfg(feature = "async")]
    pub fn split<'a, W: embedded_io_async::Write, D: Digester>(
        &'a self,
        writer: W,
        digester: D,
        config: Config,
        buf: &'a mut [u8],
    ) -> (
        Ingress<'a, D, Urc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>,
        crate::asynch::Client<'a, W, INGRESS_BUF_SIZE>,
    ) {
        (
            Ingress::new(
                digester,
                self.res_channel.publisher().unwrap(),
                self.urc_channel.publisher(),
            ),
            crate::asynch::Client::new(writer, &self.res_channel, config, buf),
        )
    }

    pub fn split_blocking<'a, W: Write, D: Digester>(
        &'a self,
        writer: W,
        digester: D,
        config: Config,
        buf: &'a mut [u8],
    ) -> (
        Ingress<D, Urc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>,
        crate::blocking::Client<W, INGRESS_BUF_SIZE>,
    ) {
        (
            Ingress::new(
                digester,
                self.res_channel.publisher().unwrap(),
                self.urc_channel.publisher(),
            ),
            crate::blocking::Client::new(writer, &self.res_channel, config, buf),
        )
    }
}
