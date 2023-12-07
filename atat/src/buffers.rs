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
        ingress_buf: &'a mut [u8; INGRESS_BUF_SIZE],
    ) -> (
        Ingress<D, Urc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>,
        crate::asynch::Client<W, INGRESS_BUF_SIZE>,
    ) {
        (
            Ingress::new(
                digester,
                self.res_channel.publisher().unwrap(),
                self.urc_channel.publisher(),
                ingress_buf,
            ),
            crate::asynch::Client::new(writer, &self.res_channel, config),
        )
    }

    pub fn split_blocking<'a, W: Write, D: Digester>(
        &'a self,
        writer: W,
        digester: D,
        config: Config,
        ingress_buf: &'a mut [u8; INGRESS_BUF_SIZE],
    ) -> (
        Ingress<D, Urc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>,
        crate::blocking::Client<W, INGRESS_BUF_SIZE>,
    ) {
        (
            Ingress::new(
                digester,
                self.res_channel.publisher().unwrap(),
                self.urc_channel.publisher(),
                ingress_buf,
            ),
            crate::blocking::Client::new(writer, &self.res_channel, config),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Buffers;
    use crate as atat;
    use crate::atat_derive::AtatUrc;
    use crate::Config;
    use crate::DefaultDigester;
    use embassy_sync::blocking_mutex::raw::NoopRawMutex;
    use embassy_sync::pipe::Pipe;
    use static_cell::StaticCell;

    #[test]
    fn static_buffer() {
        #[derive(Clone, AtatUrc)]
        pub enum Urc {
            #[at_urc("+UMWI")]
            MessageWaitingIndication,
        }

        static BUFFER: Buffers<Urc, 256, 1, 1> = Buffers::new();
        let mut pipe: Pipe<NoopRawMutex, 256> = Pipe::new();
        let (rx, tx) = pipe.split();
        static INGRESS_BUF: StaticCell<[u8; 256]> = StaticCell::new();
        let ingress_buf = INGRESS_BUF.init([0; 256]);

        let (ingress, client) = BUFFER.split(
            tx,
            DefaultDigester::<Urc>::default(),
            Config::default(),
            ingress_buf,
        );
    }
}
