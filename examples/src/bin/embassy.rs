#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![allow(incomplete_features)]

use atat::{
    asynch::{AtatClient, Client},
    AtatIngress, DefaultDigester, Ingress, ResponseSlot, UrcChannel,
};
use atat_examples::common;
use embassy_executor::Spawner;
use embassy_executor::_export::StaticCell;
use embassy_rp::{
    interrupt,
    peripherals::UART0,
    uart::{self, BufferedUart, BufferedUartRx},
};
use {defmt_rtt as _, panic_probe as _};

macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: StaticCell<T> = StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}

const INGRESS_BUF_SIZE: usize = 1024;
const URC_CAPACITY: usize = 128;
const URC_SUBSCRIBERS: usize = 3;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let (tx_pin, rx_pin, uart) = (p.PIN_0, p.PIN_1, p.UART0);

    let irq = interrupt::take!(UART0_IRQ);
    let tx_buf = &mut singleton!([0u8; 16])[..];
    let rx_buf = &mut singleton!([0u8; 16])[..];
    let uart = BufferedUart::new(
        uart,
        irq,
        tx_pin,
        rx_pin,
        tx_buf,
        rx_buf,
        uart::Config::default(),
    );
    let (reader, writer) = uart.split();

    static RES_SLOT: ResponseSlot<INGRESS_BUF_SIZE> = ResponseSlot::new();
    static URC_CHANNEL: UrcChannel<common::Urc, URC_CAPACITY, URC_SUBSCRIBERS> = UrcChannel::new();
    let ingress = Ingress::new(
        DefaultDigester::<common::Urc>::default(),
        &RES_SLOT,
        &URC_CHANNEL,
    );
    let buf = StaticCell::make_static!([0; 1024]);
    let mut client = Client::new(writer, &RES_SLOT, buf, atat::Config::default());

    spawner.spawn(ingress_task(ingress, reader)).unwrap();

    let mut state: u8 = 0;
    loop {
        // These will all timeout after 1 sec, as there is no response
        match state {
            0 => {
                client.send(&common::general::GetManufacturerId).await.ok();
            }
            1 => {
                client.send(&common::general::GetModelId).await.ok();
            }
            2 => {
                client.send(&common::general::GetSoftwareVersion).await.ok();
            }
            3 => {
                client.send(&common::general::GetWifiMac).await.ok();
            }
            _ => cortex_m::asm::bkpt(),
        }

        embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;

        state += 1;
    }
}

#[embassy_executor::task]
async fn ingress_task(
    mut ingress: Ingress<
        'static,
        DefaultDigester<common::Urc>,
        common::Urc,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
        URC_SUBSCRIBERS,
    >,
    mut reader: BufferedUartRx<'static, UART0>,
) -> ! {
    ingress.read_from(&mut reader).await
}
