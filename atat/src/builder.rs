use heapless::ArrayLength;

use crate::{
    digest::{DefaultDigester, Digester},
    queues::UrcItem,
    urc_matcher::{DefaultUrcMatcher, UrcMatcher},
    Client, Config, IngressManager, Queues,
};

type ClientParser<Tx, T, U, D, BufLen, UrcCapacity> = (
    Client<Tx, T, BufLen, UrcCapacity>,
    IngressManager<BufLen, D, U, UrcCapacity>,
);

/// Builder to set up a [`Client`] and [`IngressManager`] pair.
///
/// Create a new builder through the [`new`] method.
///
/// [`Client`]: struct.Client.html
/// [`IngressManager`]: struct.IngressManager.html
/// [`new`]: #method.new
pub struct ClientBuilder<Tx, T, U, D, BufLen, UrcCapacity>
where
    Tx: embedded_hal::serial::Write<u8>,
    T: embedded_hal::timer::CountDown,
    T::Time: From<u32>,
    U: UrcMatcher,
    D: Digester,
    BufLen: ArrayLength<u8>,
    UrcCapacity: ArrayLength<UrcItem<BufLen>>,
{
    serial_tx: Tx,
    timer: T,
    config: Config,
    custom_urc_matcher: U,
    custom_digester: D,
    #[doc(hidden)]
    _internal: core::marker::PhantomData<(BufLen, UrcCapacity)>,
}

impl<Tx, T, BufLen, UrcCapacity>
    ClientBuilder<Tx, T, DefaultUrcMatcher, DefaultDigester, BufLen, UrcCapacity>
where
    Tx: embedded_hal::serial::Write<u8>,
    T: embedded_hal::timer::CountDown,
    T::Time: From<u32>,
    BufLen: ArrayLength<u8>,
    UrcCapacity: ArrayLength<UrcItem<BufLen>>,
{
    /// Create a builder for new Atat client instance.
    ///
    /// The `serial_tx` type must implement the `embedded_hal`
    /// [`serial::Write<u8>`][serialwrite] trait while the timer must implement
    /// the [`timer::CountDown`][timercountdown] trait.
    ///
    /// [serialwrite]: ../embedded_hal/serial/trait.Write.html
    /// [timercountdown]: ../embedded_hal/timer/trait.CountDown.html
    pub fn new(serial_tx: Tx, timer: T, config: Config) -> Self {
        Self {
            serial_tx,
            timer,
            config,
            custom_urc_matcher: DefaultUrcMatcher::default(),
            custom_digester: DefaultDigester::default(),
            #[doc(hidden)]
            _internal: core::marker::PhantomData,
        }
    }
}

impl<Tx, T, U, D, BufLen, UrcCapacity> ClientBuilder<Tx, T, U, D, BufLen, UrcCapacity>
where
    Tx: embedded_hal::serial::Write<u8>,
    T: embedded_hal::timer::CountDown,
    T::Time: From<u32>,
    U: UrcMatcher,
    D: Digester,
    BufLen: ArrayLength<u8>,
    UrcCapacity: ArrayLength<UrcItem<BufLen>>,
{
    /// Use a custom [`UrcMatcher`] implementation.
    ///
    /// [`UrcMatcher`]: trait.UrcMatcher.html
    pub fn with_custom_urc_matcher<U2: UrcMatcher>(
        self,
        matcher: U2,
    ) -> ClientBuilder<Tx, T, U2, D, BufLen, UrcCapacity> {
        ClientBuilder {
            serial_tx: self.serial_tx,
            timer: self.timer,
            config: self.config,
            custom_urc_matcher: matcher,
            custom_digester: self.custom_digester,
            #[doc(hidden)]
            _internal: self._internal,
        }
    }

    /// Use a custom [`Digester`] implementation.
    ///
    /// [`Digester`]: trait.Digester.html
    pub fn with_custom_digester<D2: Digester>(
        self,
        digester: D2,
    ) -> ClientBuilder<Tx, T, U, D2, BufLen, UrcCapacity> {
        ClientBuilder {
            custom_urc_matcher: self.custom_urc_matcher,
            serial_tx: self.serial_tx,
            timer: self.timer,
            config: self.config,
            custom_digester: digester,
            #[doc(hidden)]
            _internal: self._internal,
        }
    }

    /// Set up and return a [`Client`] and [`IngressManager`] pair.
    ///
    /// [`Client`]: struct.Client.html
    /// [`IngressManager`]: struct.IngressManager.html
    pub fn build(
        self,
        queues: Queues<BufLen, UrcCapacity>,
    ) -> ClientParser<Tx, T, U, D, BufLen, UrcCapacity> {
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
