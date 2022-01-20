#![no_main]
#![no_std]

use bbqueue::BBBuffer;
use stm32l4xx_hal as hal;

mod common;

use cortex_m::asm;
use hal::{
    pac::{interrupt, Peripherals, USART2},
    prelude::*,
    serial::{Config, Event::Rxne, Rx, Serial},
    timer::{Event, Timer},
};

use atat::{AtatClient, ClientBuilder, Clock, ComQueue, Queues};

use heapless::spsc::Queue;

use cortex_m_rt::entry;

struct AtClock<TIM, const TIMER_HZ: u32> {
    _timer: Timer<TIM>,
}

impl<TIM, const TIMER_HZ: u32> AtClock<TIM, TIMER_HZ> {
    fn new(timer: Timer<TIM>) -> Self {
        Self { _timer: timer }
    }
}

impl<TIM, const TIMER_HZ: u32> Clock<TIMER_HZ> for AtClock<TIM, TIMER_HZ> {
    type Error = core::convert::Infallible;

    fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ> {
        fugit::TimerInstantU32::from_ticks(0)
    }

    fn start(&mut self, _duration: fugit::TimerDurationU32<TIMER_HZ>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        Ok(())
    }
}

static mut INGRESS: Option<
    atat::IngressManager<atat::DefaultDigester, atat::DefaultUrcMatcher, 256, 1024, 512>,
> = None;
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
    let at_clock: AtClock<_, 100> = AtClock::new(at_timer);

    let mut serial = Serial::usart2(
        p.USART2,
        (tx, rx),
        Config::default().baudrate(115_200.bps()),
        clocks,
        &mut rcc.apb1r1,
    );

    serial.listen(Rxne);

    static mut RES_QUEUE: BBBuffer<1024> = BBBuffer::new();
    static mut URC_QUEUE: BBBuffer<512> = BBBuffer::new();
    static mut COM_QUEUE: ComQueue = Queue::new();

    let queues = Queues {
        res_queue: unsafe { RES_QUEUE.try_split_framed().unwrap() },
        urc_queue: unsafe { URC_QUEUE.try_split_framed().unwrap() },
        com_queue: unsafe { COM_QUEUE.split() },
    };

    let (tx, rx) = serial.split();
    let (mut client, ingress) =
        ClientBuilder::new(tx, at_clock, atat::Config::new(atat::Mode::Timeout)).build(queues);

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
            Ok(_response) => {
                // Do something with response here
            }
            Err(_e) => {}
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

#[panic_handler] // panicking behavior
fn panic(_info: &core::panic::PanicInfo) -> ! {
    cortex_m::peripheral::SCB::sys_reset();
}
