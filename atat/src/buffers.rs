use bbqueue::BBBuffer;

use crate::{Config, Digester};

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
    pub fn split<
        'a,
        W: embedded_io::asynch::Write,
        Clk: embedded_time::Clock,
        Delay: embedded_hal_async::delay::DelayUs,
        D: Digester,
    >(
        &'a self,
        writer: W,
        clock: &'a Clk,
        delay: Delay,
        digester: D,
        config: Config,
    ) -> (
        crate::asynch::Ingress<'a, D, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY>,
        crate::asynch::Client<'a, W, Clk, Delay, RES_CAPACITY, URC_CAPACITY>,
    ) {
        let (res_writer, res_reader) = self.res_queue.try_split_framed().unwrap();
        let (urc_writer, urc_reader) = self.urc_queue.try_split_framed().unwrap();

        (
            crate::asynch::Ingress::new(digester, res_writer, urc_writer),
            crate::asynch::Client::new(writer, clock, delay, res_reader, urc_reader, config),
        )
    }

    // pub fn split_blocking<
    //     'a,
    //     W: embedded_io::blocking::Write,
    //     Clk: embedded_time::Clock,
    //     Delay: embedded_hal_async::delay::DelayUs,
    //     D: Digester,
    // >(
    //     &'a self,
    //     writer: W,
    //     clock: &'a Clk,
    //     delay: Delay,
    //     digester: D,
    //     config: Config,
    // ) -> (
    //     crate::blocking::Ingress<'a, D, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY>,
    //     crate::blocking::Client<'a, W, Clk, Delay, RES_CAPACITY, URC_CAPACITY>,
    // ) {
    //     let (res_writer, res_reader) = self.res_queue.try_split_framed().unwrap();
    //     let (urc_writer, urc_reader) = self.urc_queue.try_split_framed().unwrap();

    //     (
    //         crate::blocking::Ingress::new(digester, res_writer, urc_writer),
    //         crate::blocking::Client::new(writer, clock, delay, res_reader, urc_reader, config),
    //     )
    // }
}
