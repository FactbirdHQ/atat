use bbqueue::BBBuffer;
use embedded_io::blocking::Write;

use crate::{Config, Digester, Ingress};

pub struct Buffers<
    const INGRESS_BUF_SIZE: usize,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
> {
    res_queue: BBBuffer<RES_CAPACITY>,
    urc_queue: BBBuffer<URC_CAPACITY>,
}

impl<const INGRESS_BUF_SIZE: usize, const RES_CAPACITY: usize, const URC_CAPACITY: usize>
    Buffers<INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY>
{
    pub const fn new() -> Self {
        Self {
            res_queue: BBBuffer::new(),
            urc_queue: BBBuffer::new(),
        }
    }

    #[cfg(feature = "async")]
    pub fn split<'a, W: embedded_io::asynch::Write, D: Digester>(
        &'a self,
        writer: W,
        digester: D,
        config: Config,
    ) -> (
        Ingress<'a, D, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY>,
        crate::asynch::Client<'a, W, RES_CAPACITY, URC_CAPACITY>,
    ) {
        let (res_writer, res_reader) = self.res_queue.try_split_framed().unwrap();
        let (urc_writer, urc_reader) = self.urc_queue.try_split_framed().unwrap();

        (
            Ingress::new(digester, res_writer, urc_writer),
            crate::asynch::Client::new(writer, res_reader, urc_reader, config),
        )
    }

    pub fn split_blocking<'a, W: Write, D: Digester>(
        &'a self,
        writer: W,
        digester: D,
        config: Config,
    ) -> (
        Ingress<'a, D, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY>,
        crate::blocking::Client<'a, W, RES_CAPACITY, URC_CAPACITY>,
    ) {
        let (res_writer, res_reader) = self.res_queue.try_split_framed().unwrap();
        let (urc_writer, urc_reader) = self.urc_queue.try_split_framed().unwrap();

        (
            Ingress::new(digester, res_writer, urc_writer),
            crate::blocking::Client::new(writer, res_reader, urc_reader, config),
        )
    }
}
