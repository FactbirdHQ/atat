#![no_main]
#![no_std]
mod common;

use defmt_rtt as _; // global logger
use embassy_stm32::peripherals::USART3;
use embassy_stm32::{dma::NoDma, gpio};
use panic_probe as _;

use atat::{clock::Clock, AtatClient, ClientBuilder, Queues};
use bbqueue::BBBuffer;
use common::{timer::DwtTimer, Urc};
use cortex_m_rt::entry;
use fugit::ExtU32;

use embedded_hal_nb::nb;

#[cfg(feature = "defmt")]
defmt::timestamp!("{=u64}", { DwtTimer::<80_000_000>::now() / 80_000 });

const RX_BUFFER_BYTES: usize = 512;
// Response queue is capable of holding one full RX buffer
const RES_CAPACITY_BYTES: usize = RX_BUFFER_BYTES;
// URC queue is capable of holding up to three full RX buffer
const URC_CAPACITY_BYTES: usize = RX_BUFFER_BYTES * 3;

static mut INGRESS: Option<
    atat::IngressManager<
        atat::AtDigester<Urc>,
        RX_BUFFER_BYTES,
        RES_CAPACITY_BYTES,
        URC_CAPACITY_BYTES,
    >,
> = None;
static mut RX: Option<embassy_stm32::usart::UartRx<USART3>> = None;
// static mut TIMER: Option<Timer<TIM7>> = None;

#[entry]
fn main() -> ! {
    // Create static queues for ATAT
    static mut RES_QUEUE: BBBuffer<RES_CAPACITY_BYTES> = BBBuffer::new();
    static mut URC_QUEUE: BBBuffer<URC_CAPACITY_BYTES> = BBBuffer::new();

    let p = embassy_stm32::init(Default::default());

    let mut wifi_nrst = gpio::OutputOpenDrain::new(
        p.PD13,
        gpio::Level::Low,
        gpio::Speed::Medium,
        gpio::Pull::None,
    );
    wifi_nrst.set_high();

    let mut serial = embassy_stm32::usart::Uart::new(
        p.USART3,
        p.PD9,
        p.PD8,
        // p.PB1,
        // p.PA6,
        NoDma,
        NoDma,
        embassy_stm32::usart::Config::default(),
    );

    let (tx, rx) = serial.split();

    // Instantiate ATAT client & IngressManager
    let queues = Queues {
        res_queue: RES_QUEUE.try_split_framed().unwrap(),
        urc_queue: URC_QUEUE.try_split_framed().unwrap(),
    };

    let (mut client, ingress) = ClientBuilder::new(
        tx,
        DwtTimer::<80_000_000>::new(),
        atat::AtDigester::new(),
        atat::Config::new(atat::Mode::Timeout),
    )
    .build(queues);

    // configure NVIC interrupts
    // let mut timer = Timer::tim7(p.TIM7, 100.hz(), clocks, &mut rcc.apb1r1);
    // unsafe { cortex_m::peripheral::NVIC::unmask(hal::stm32::Interrupt::TIM7) };
    unsafe { cortex_m::peripheral::NVIC::unmask(embassy_stm32::pac::Interrupt::USART3) };
    // timer.listen(Event::TimeOut);

    unsafe { INGRESS = Some(ingress) };
    unsafe { RX = Some(rx) };
    // unsafe { TIMER = Some(timer) };

    let mut state = 0;
    let mut loop_timer = DwtTimer::<80_000_000>::new();

    loop {
        #[cfg(feature = "defmt")]
        defmt::debug!("\r\n\r\n\r\n");

        match state {
            0 => {
                client.send(&common::general::GetManufacturerId).ok();
            }
            1 => {
                client.send(&common::general::GetModelId).ok();
            }
            2 => {
                client.send(&common::general::GetSoftwareVersion).ok();
            }
            3 => {
                client.send(&common::general::GetWifiMac).ok();
            }
            _ => cortex_m::asm::bkpt(),
        }

        loop_timer.start(1.secs()).ok();
        nb::block!(loop_timer.wait()).ok();

        state += 1;
    }
}

// #[interrupt]
// fn TIM7() {
//     cortex_m::interrupt::free(|_| {
//         let timer = unsafe { TIMER.as_mut().unwrap() };
//         timer.clear_update_interrupt_flag();
//         let ingress = unsafe { INGRESS.as_mut().unwrap() };
//         ingress.digest();
//     });
// }

#[cortex_m_rt::interrupt]
fn USART3() {
    cortex_m::interrupt::free(|_| {
        let ingress = unsafe { INGRESS.as_mut().unwrap() };
        let rx = unsafe { RX.as_mut().unwrap() };
        if let Ok(d) = nb::block!(rx.nb_read()) {
            ingress.write(&[d]);
        }
    });
}

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
