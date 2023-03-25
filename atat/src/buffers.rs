use embedded_io::blocking::Write;

use crate::{reschannel::ResChannel, urchannel::UrcChannel, AtatUrc, Config, Digester, Ingress};

pub struct Buffers<
    Urc: AtatUrc,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
    const URC_SUBSCRIBERS: usize,
> {
    res_channel: ResChannel<INGRESS_BUF_SIZE>,
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
            res_channel: ResChannel::new(),
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
    pub fn split<W: embedded_io::asynch::Write, D: Digester>(
        &self,
        writer: W,
        digester: D,
        config: Config,
    ) -> (
        Ingress<'_, D, Urc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>,
        crate::asynch::Client<'_, W, INGRESS_BUF_SIZE>,
    ) {
        (
            Ingress::new(
                digester,
                self.res_channel.publisher().unwrap(),
                self.urc_channel.publisher(),
            ),
            crate::asynch::Client::new(writer, &self.res_channel, config),
        )
    }

    pub fn split_blocking<W: Write, D: Digester>(
        &self,
        writer: W,
        digester: D,
        config: Config,
    ) -> (
        Ingress<'_, D, Urc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>,
        crate::blocking::Client<'_, W, INGRESS_BUF_SIZE>,
    ) {
        (
            Ingress::new(
                digester,
                self.res_channel.publisher().unwrap(),
                self.urc_channel.publisher(),
            ),
            crate::blocking::Client::new(writer, &self.res_channel, config),
        )
    }
}
