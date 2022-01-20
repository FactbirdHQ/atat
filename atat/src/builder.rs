use crate::{
    digest::{DefaultDigester, Digester},
    urc_matcher::{DefaultUrcMatcher, UrcMatcher},
    Client, Config, IngressManager, Queues,
};

type ClientParser<
    Tx,
    CLK,
    U,
    D,
    const TIMER_HZ: u32,
    const BUF_LEN: usize,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
> = (
    Client<Tx, CLK, TIMER_HZ, RES_CAPACITY, URC_CAPACITY>,
    IngressManager<D, U, BUF_LEN, RES_CAPACITY, URC_CAPACITY>,
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
    CLK,
    U,
    D,
    const TIMER_HZ: u32,
    const BUF_LEN: usize,
    const RES_CAPACITY: usize,
    const URC_CAPACITY: usize,
> where
    Tx: embedded_hal::serial::nb::Write<u8>,
    CLK: super::Clock<TIMER_HZ>,
    U: UrcMatcher,
    D: Digester,
{
    serial_tx: Tx,
    timer: CLK,
    config: Config,
    custom_urc_matcher: U,
    custom_digester: D,
}

impl<
        Tx,
        T,
        const TIMER_HZ: u32,
        const BUF_LEN: usize,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
    >
    ClientBuilder<
        Tx,
        T,
        DefaultUrcMatcher,
        DefaultDigester,
        TIMER_HZ,
        BUF_LEN,
        RES_CAPACITY,
        URC_CAPACITY,
    >
where
    Tx: embedded_hal::serial::nb::Write<u8>,
    T: super::Clock<TIMER_HZ>,
{
    /// Create a builder for new Atat client instance.
    ///
    /// The `serial_tx` type must implement the `embedded_hal`
    /// [`serial::Write<u8>`][serialwrite] trait while the timer must implement
    /// the [`Clock`][clock] trait.
    ///
    /// [serialwrite]: ../embedded_hal/serial/trait.Write.html
    /// [clock]: trait.Clock.html
    pub fn new(serial_tx: Tx, timer: T, config: Config) -> Self {
        Self {
            serial_tx,
            timer,
            config,
            custom_urc_matcher: DefaultUrcMatcher::default(),
            custom_digester: DefaultDigester::default(),
        }
    }
}

impl<
        Tx,
        CLK,
        U,
        D,
        const TIMER_HZ: u32,
        const BUF_LEN: usize,
        const RES_CAPACITY: usize,
        const URC_CAPACITY: usize,
    > ClientBuilder<Tx, CLK, U, D, TIMER_HZ, BUF_LEN, RES_CAPACITY, URC_CAPACITY>
where
    Tx: embedded_hal::serial::nb::Write<u8>,
    CLK: super::Clock<TIMER_HZ>,
    U: UrcMatcher,
    D: Digester,
{
    /// Use a custom [`UrcMatcher`] implementation.
    ///
    /// [`UrcMatcher`]: trait.UrcMatcher.html
    pub fn with_custom_urc_matcher<U2: UrcMatcher>(
        self,
        matcher: U2,
    ) -> ClientBuilder<Tx, CLK, U2, D, TIMER_HZ, BUF_LEN, RES_CAPACITY, URC_CAPACITY> {
        ClientBuilder {
            serial_tx: self.serial_tx,
            timer: self.timer,
            config: self.config,
            custom_urc_matcher: matcher,
            custom_digester: self.custom_digester,
        }
    }

    /// Use a custom [`Digester`] implementation.
    ///
    /// [`Digester`]: trait.Digester.html
    pub fn with_custom_digester<D2: Digester>(
        self,
        digester: D2,
    ) -> ClientBuilder<Tx, CLK, U, D2, TIMER_HZ, BUF_LEN, RES_CAPACITY, URC_CAPACITY> {
        ClientBuilder {
            custom_urc_matcher: self.custom_urc_matcher,
            serial_tx: self.serial_tx,
            timer: self.timer,
            config: self.config,
            custom_digester: digester,
        }
    }

    /// Set up and return a [`Client`] and [`IngressManager`] pair.
    ///
    /// [`Client`]: struct.Client.html
    /// [`IngressManager`]: struct.IngressManager.html
    pub fn build(
        self,
        queues: Queues<RES_CAPACITY, URC_CAPACITY>,
    ) -> ClientParser<Tx, CLK, U, D, TIMER_HZ, BUF_LEN, RES_CAPACITY, URC_CAPACITY> {
        let parser = IngressManager::with_customs(
            queues.res_queue.0,
            queues.urc_queue.0,
            queues.com_queue.1,
            self.custom_urc_matcher,
            self.custom_digester,
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
