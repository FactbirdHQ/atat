use crate::{Config, Digester};

use super::{Client, Ingress};
use bbqueue::BBBuffer;
use embedded_hal_async::delay::DelayUs;
use embedded_io::asynch::Write;
use embedded_time::Clock;

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

    pub fn split<'a, W: Write, Clk: Clock, Delay: DelayUs, D: Digester>(
        &'a self,
        writer: W,
        clock: &'a Clk,
        delay: Delay,
        digester: D,
        config: Config,
    ) -> (
        Ingress<'a, D, INGRESS_BUF_SIZE, RES_CAPACITY, URC_CAPACITY>,
        Client<'a, W, Clk, Delay, RES_CAPACITY, URC_CAPACITY>,
    ) {
        let (res_writer, res_reader) = self.res_queue.try_split_framed().unwrap();
        let (urc_writer, urc_reader) = self.urc_queue.try_split_framed().unwrap();

        (
            Ingress::new(digester, res_writer, urc_writer),
            Client::new(writer, clock, delay, res_reader, urc_reader, config),
        )
    }
}
