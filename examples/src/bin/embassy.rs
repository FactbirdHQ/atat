#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use atat::{
    asynch::{AtatClient, Client},
    AtatIngress, DefaultDigester, Ingress, ResponseSlot, UrcChannel,
};
use atat_examples::common;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::UART0,
    uart::{self, BufferedInterruptHandler, BufferedUart, BufferedUartRx},
};
use {defmt_rtt as _, panic_probe as _};

const INGRESS_BUF_SIZE: usize = 1024;
const URC_CAPACITY: usize = 128;
const URC_SUBSCRIBERS: usize = 3;

bind_interrupts!(struct Irqs {
    UART0_IRQ => BufferedInterruptHandler<UART0>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let (tx_pin, rx_pin, uart) = (p.PIN_0, p.PIN_1, p.UART0);

    let tx_buf = static_cell::make_static!([0u8; 16]);
    let rx_buf = static_cell::make_static!([0u8; 16]);
    let uart = BufferedUart::new(
        uart,
        Irqs,
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
    let buf = static_cell::make_static!([0; 1024]);
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
