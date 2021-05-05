#![no_main]
#![no_std]

extern crate atat;
extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate heapless;
extern crate nb;

#[cfg(not(test))]
extern crate panic_halt;
extern crate stm32l4xx_hal as hal;

mod common;

use cortex_m::asm;
use hal::{
    pac::{interrupt, Peripherals, USART2},
    prelude::*,
    serial::{Config, Event::Rxne, Rx, Serial},
    timer::{Event, Timer},
};

use atat::{
    digest::DefaultDigester, urc_matcher::DefaultUrcMatcher, ClientBuilder, ComQueue, Queues,
    ResQueue, UrcQueue,
};

use heapless::{consts, spsc::Queue};

use crate::rt::entry;

static mut INGRESS: Option<atat::IngressManager<consts::U256>> = None;
static mut RX: Option<Rx<USART2>> = None;

#[entry]
fn main() -> ! {
    let p = Peripherals::take().unwrap();

    let mut flash = p.FLASH.constrain();
    let mut rcc = p.RCC.constrain();
    let mut pwr = p.PWR.constrain(&mut rcc.apb1r1);

    let mut gpioa = p.GPIOA.split(&mut rcc.ahb2);
    // let mut gpiob = p.GPIOB.split(&mut rcc.ahb2);

    // clock configuration using the default settings (all clocks run at 8 MHz)
    let clocks = rcc.cfgr.freeze(&mut flash.acr, &mut pwr);
    // TRY this alternate clock configuration (clocks run at nearly the maximum frequency)
    // let clocks = rcc.cfgr.sysclk(64.mhz()).pclk1(32.mhz()).freeze(&mut flash.acr);

    // The Serial API is highly generic
    // TRY the commented out, different pin configurations
    // let tx = gpioa.pa9.into_af7(&mut gpioa.moder, &mut gpioa.afrh);
    // let tx = gpiob.pb6.into_af7(&mut gpiob.moder, &mut gpiob.afrl);

    // let rx = gpioa.pa10.into_af7(&mut gpioa.moder, &mut gpioa.afrh);
    // let rx = gpiob.pb7.into_af7(&mut gpiob.moder, &mut gpiob.afrl);

    let tx = gpioa.pa2.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
    let rx = gpioa.pa3.into_af7(&mut gpioa.moder, &mut gpioa.afrl);

    let mut timer = Timer::tim7(p.TIM7, 1.hz(), clocks, &mut rcc.apb1r1);
    let at_timer = Timer::tim6(p.TIM6, 100.hz(), clocks, &mut rcc.apb1r1);

    let mut serial = Serial::usart2(
        p.USART2,
        (tx, rx),
        Config::default().baudrate(115_200.bps()),
        clocks,
        &mut rcc.apb1r1,
    );

    serial.listen(Rxne);

    static mut RES_QUEUE: ResQueue<consts::U256> = Queue(heapless::i::Queue::u8());
    static mut URC_QUEUE: UrcQueue<consts::U256, consts::U10> = Queue(heapless::i::Queue::u8());
    static mut COM_QUEUE: ComQueue = Queue(heapless::i::Queue::u8());

    let queues = Queues {
        res_queue: unsafe { RES_QUEUE.split() },
        urc_queue: unsafe { URC_QUEUE.split() },
        com_queue: unsafe { COM_QUEUE.split() },
    };

    let (tx, rx) = serial.split();
    let (mut client, ingress) =
        ClientBuilder::new(tx, at_timer, atat::Config::new(atat::Mode::Timeout)).build(queues);

    unsafe { INGRESS = Some(ingress) };
    unsafe { RX = Some(rx) };

    // configure NVIC interrupts
    unsafe { cortex_m::peripheral::NVIC::unmask(hal::stm32::Interrupt::TIM7) };
    timer.listen(Event::TimeOut);

    // if all goes well you should reach this breakpoint
    asm::bkpt();

    loop {
        asm::wfi();

        match client.send(&common::AT) {
            Ok(response) => {
                // Do something with response here
            }
            Err(e) => {}
        }
    }
}

#[interrupt]
fn TIM7() {
    let ingress = unsafe { INGRESS.as_mut().unwrap() };
    ingress.digest();
}

#[interrupt]
fn USART2() {
    let ingress = unsafe { INGRESS.as_mut().unwrap() };
    let rx = unsafe { RX.as_mut().unwrap() };
    if let Ok(d) = nb::block!(rx.read()) {
        ingress.write(&[d]);
    }
}
