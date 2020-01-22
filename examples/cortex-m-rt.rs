// #![deny(warnings)]
#![no_main]
#![no_std]


extern crate at_rs as at;
extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate heapless;
extern crate nb;
extern crate panic_halt;
extern crate stm32l4xx_hal as hal;

mod common;
use common::command::{Command, Response};

use crate::hal::{
    gpio::{
        gpioa::{PA2, PA3},
        Alternate, Floating, Input, AF7,
    },
    prelude::*,
    serial::{Config, Serial},
    stm32::{self, interrupt},
    timer::{Event, Timer},
};

use crate::rt::entry;
use cortex_m::asm;
use heapless::{consts, spsc::Queue};

use at::ATParser;
use at::Error as ATError;

type SerialUSART2 = Serial<
    stm32::USART2,
    (
        PA2<Alternate<AF7, Input<Floating>>>,
        PA3<Alternate<AF7, Input<Floating>>>,
    ),
>;


static mut REQ_Q: Option<Queue<Command, consts::U5, u8>> = None;
static mut RES_Q: Option<Queue<Result<Response, ATError>, consts::U5, u8>> = None;
static mut AT_PARSER: Option<
    ATParser<SerialUSART2, Command, consts::U1024, consts::U5, consts::U5>,
> = None;

#[entry]
fn main() -> ! {
    let p = stm32::Peripherals::take().unwrap();

    let mut flash = p.FLASH.constrain();
    let mut rcc = p.RCC.constrain();

    let mut gpioa = p.GPIOA.split(&mut rcc.ahb2);
    // let mut gpiob = p.GPIOB.split(&mut rcc.ahb2);

    // clock configuration using the default settings (all clocks run at 8 MHz)
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
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

    unsafe { REQ_Q = Some(Queue::u8()) };
    unsafe { RES_Q = Some(Queue::u8()) };

    // TRY using a different USART peripheral here
    // let serial = Serial::usart1(p.USART1, (tx, rx), 9_600.bps(), clocks, &mut rcc.apb2);
    let serial = Serial::usart2(
        p.USART2,
        (tx, rx),
        Config::default().baudrate(115_200.bps()),
        clocks,
        &mut rcc.apb1r1,
    );
    let tim = Timer::tim6(p.TIM6, 100.hz(), clocks, &mut rcc.apb1r1);

    let (at_client, at_parser) = at::new(
        unsafe { (REQ_Q.as_mut().unwrap(), RES_Q.as_mut().unwrap()) },
        serial,
        tim,
        1.hz(),
    );

    let (mut cmd_p, mut resp_c) = at_client.release();

    unsafe { AT_PARSER = Some(at_parser) };
    // configure NVIC interrupts
    unsafe { cortex_m::peripheral::NVIC::unmask(hal::stm32::Interrupt::TIM7) };

    // Adjust this spin rate to set how often the request/response queue is checked
    let mut timer = Timer::tim7(p.TIM7, 100.hz(), clocks, &mut rcc.apb1r1);
    timer.listen(Event::TimeOut);

    // if all goes well you should reach this breakpoint
    asm::bkpt();

    cmd_p.enqueue(Command::AT).unwrap();

    loop {
        asm::wfi();

        if let Some(_response) = resp_c.dequeue() {
            // Do something with response here
        }
    }
}

#[interrupt]
fn TIM7() {
    let at_parser = unsafe { AT_PARSER.as_mut().unwrap() };
    at_parser.spin();
}

#[interrupt]
fn USART1() {
    let at_parser = unsafe { AT_PARSER.as_mut().unwrap() };
    at_parser.handle_irq();
}
