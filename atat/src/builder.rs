use crate::{nom_digest::Digester, Client, Config, IngressManager, Queues};

type ClientParser<
    Tx,
    T,
    D,
    const TIMER_HZ: u32,
    const BUF_LEN: usize,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
> = (
    Client<Tx, T, TIMER_HZ, RES_CAPACITY, URC_CAPACITY>,
    IngressManager<D, BUF_LEN, RES_CAPACITY, URC_CAPACITY>,
);

/// Builder to set up a [`Client`] and [`IngressManager`] pair.
///
/// Create a new builder through the [`new`] method.
///
/// [`Client`]: struct.Client.html
/// [`IngressManager`]: struct.IngressManager.html
/// [`new`]: #method.new
pub struct ClientBuilder<
    Tx,
    T,
    D,
    const TIMER_HZ: u32,
    const BUF_LEN: usize,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
> where
    Tx: embedded_hal::serial::nb::Write<u8>,
    T: fugit_timer::Timer<TIMER_HZ>,
    D: Digester,
{
    serial_tx: Tx,
    timer: T,
    config: Config,
    digester: D,
}

impl<
        Tx,
        T,
        D,
        const TIMER_HZ: u32,
        const BUF_LEN: usize,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
    > ClientBuilder<Tx, T, D, TIMER_HZ, BUF_LEN, RES_CAPACITY, URC_CAPACITY>
where
    Tx: embedded_hal::serial::nb::Write<u8>,
    T: fugit_timer::Timer<TIMER_HZ>,
    D: Digester,
{
    /// Create a builder for new Atat client instance.
    ///
    /// The `serial_tx` type must implement the `embedded_hal`
    /// [`serial::Write<u8>`][serialwrite] trait while the timer must implement
    /// the [`fugit_timer::Timer`] trait.
    ///
    /// [serialwrite]: ../embedded_hal/serial/trait.Write.html
    pub fn new(serial_tx: Tx, timer: T, digester: D, config: Config) -> Self {
        Self {
            serial_tx,
            timer,
            config,
            digester,
        }
    }

    /// Set up and return a [`Client`] and [`IngressManager`] pair.
    ///
    /// [`Client`]: struct.Client.html
    /// [`IngressManager`]: struct.IngressManager.html
    pub fn build(
        self,
        queues: Queues<RES_CAPACITY, URC_CAPACITY>,
    ) -> ClientParser<Tx, T, D, TIMER_HZ, BUF_LEN, RES_CAPACITY, URC_CAPACITY> {
        let parser = IngressManager::new(
            queues.res_queue.0,
            queues.urc_queue.0,
            queues.com_queue.1,
            self.digester,
        );
        let client = Client::new(
            self.serial_tx,
            queues.res_queue.1,
            queues.urc_queue.1,
            queues.com_queue.0,
            self.timer,
            self.config,
        );

        (client, parser)
    }
}
