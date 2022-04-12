#![no_main]
#![no_std]
mod common;

use stm32l4xx_hal as hal;

use defmt_rtt as _;
use panic_probe as _; // global logger

use atat::{clock::Clock, AtatClient, ClientBuilder, Queues};
use bbqueue::BBBuffer;
use common::{timer::DwtTimer, Urc};
use cortex_m_rt::entry;
use fugit::ExtU32;
use hal::{
    device::{Peripherals, TIM7},
    pac::{interrupt, USART3},
    prelude::*,
    rcc::{ClockSecuritySystem, CrystalBypass, MsiFreq, PllConfig, PllDivider, PllSource},
    serial::{Config, Event::Rxne, Rx, Serial},
    timer::{Event, Timer},
};
use heapless::spsc::Queue;

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
static mut RX: Option<Rx<USART3>> = None;
static mut TIMER: Option<Timer<TIM7>> = None;

#[entry]
fn main() -> ! {
    // Create static queues for ATAT
    static mut RES_QUEUE: BBBuffer<RES_CAPACITY_BYTES> = BBBuffer::new();
    static mut URC_QUEUE: BBBuffer<URC_CAPACITY_BYTES> = BBBuffer::new();

    // Setup clocks & peripherals
    let p = Peripherals::take().unwrap();
    let mut flash = p.FLASH.constrain();
    let mut rcc = p.RCC.constrain();
    let mut pwr = p.PWR.constrain(&mut rcc.apb1r1);

    // clock configuration using the default settings (all clocks run at 8 MHz)
    let clocks = rcc
        .cfgr
        // .hsi48(true)
        .lse(CrystalBypass::Disable, ClockSecuritySystem::Disable)
        .hse(
            8.mhz(),
            CrystalBypass::Disable,
            ClockSecuritySystem::Disable,
        )
        .sysclk_with_pll(80.mhz(), PllConfig::new(1, 20, PllDivider::Div2))
        .pll_source(PllSource::HSE)
        // Temp fix until PLLSAI1 is implemented
        .msi(MsiFreq::RANGE48M)
        .hclk(80.mhz())
        .pclk1(80.mhz())
        .pclk2(80.mhz())
        .freeze(&mut flash.acr, &mut pwr);

    let mut gpioa = p.GPIOA.split(&mut rcc.ahb2);
    let mut gpiob = p.GPIOB.split(&mut rcc.ahb2);
    let mut gpiod = p.GPIOD.split(&mut rcc.ahb2);

    let mut wifi_nrst = gpiod
        .pd13
        .into_open_drain_output(&mut gpiod.moder, &mut gpiod.otyper);
    wifi_nrst.set_high();

    let tx =
        gpiod
            .pd8
            .into_alternate_push_pull(&mut gpiod.moder, &mut gpiod.otyper, &mut gpiod.afrh);
    let rx =
        gpiod
            .pd9
            .into_alternate_push_pull(&mut gpiod.moder, &mut gpiod.otyper, &mut gpiod.afrh);
    let rts =
        gpiob
            .pb1
            .into_alternate_push_pull(&mut gpiob.moder, &mut gpiob.otyper, &mut gpiob.afrl);
    let cts =
        gpioa
            .pa6
            .into_alternate_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);

    // Configure UART peripheral
    let mut serial = Serial::usart3(
        p.USART3,
        (tx, rx, rts, cts),
        Config::default().baudrate(115_200.bps()),
        clocks,
        &mut rcc.apb1r1,
    );
    serial.listen(Rxne);
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
    let mut timer = Timer::tim7(p.TIM7, 100.hz(), clocks, &mut rcc.apb1r1);
    unsafe { cortex_m::peripheral::NVIC::unmask(hal::stm32::Interrupt::TIM7) };
    unsafe { cortex_m::peripheral::NVIC::unmask(hal::stm32::Interrupt::USART3) };
    timer.listen(Event::TimeOut);

    unsafe { INGRESS = Some(ingress) };
    unsafe { RX = Some(rx) };
    unsafe { TIMER = Some(timer) };

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

#[interrupt]
fn TIM7() {
    cortex_m::interrupt::free(|_| {
        let timer = unsafe { TIMER.as_mut().unwrap() };
        timer.clear_update_interrupt_flag();
        let ingress = unsafe { INGRESS.as_mut().unwrap() };
        ingress.digest();
    });
}

#[interrupt]
fn USART3() {
    cortex_m::interrupt::free(|_| {
        let ingress = unsafe { INGRESS.as_mut().unwrap() };
        let rx = unsafe { RX.as_mut().unwrap() };
        if let Ok(d) = nb::block!(rx.read()) {
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
