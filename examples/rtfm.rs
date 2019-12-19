#![no_main]
#![no_std]
// TEMP: Crate wide allow this
#![allow(deprecated, unused)]

extern crate at_rs as at;
extern crate heapless;
extern crate panic_halt;
extern crate stm32l4xx_hal as hal;

mod common;
use common::command::{Command, Response};

use hal::{
  gpio::{
    gpioa::{PA2, PA3},
    Alternate, Floating, Input, AF7,
  },
  pac::{self, interrupt},
  prelude::*,
  serial::{Config, Event::Rxne, Serial},
  timer::{Event, Timer},
};

use rtfm::{
  app,
  cyccnt::{Duration, Instant, U32Ext},
  export::wfi,
};

use heapless::{consts, spsc::Queue};

use core::fmt::Write;

use at::Error as ATError;

const HEAP_SIZE: usize = 1024; // in bytes

type SerialUSART2 = Serial<
  pac::USART2,
  (
    PA2<Alternate<AF7, Input<Floating>>>,
    PA3<Alternate<AF7, Input<Floating>>>,
  ),
>;

#[app(device = hal::pac, peripherals = true, monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {
  struct Resources {
    at_parser: at::ATParser<SerialUSART2, Command, Response>,
  }

  #[init(spawn = [at_loop])]
  fn init(ctx: init::Context) -> init::LateResources {
    static mut CMD_Q: Option<Queue<Command, consts::U10, u8>> = None;
    static mut RESP_Q: Option<Queue<Result<Response, ATError>, consts::U10, u8>> = None;

    let p = pac::Peripherals::take().unwrap();

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

    *CMD_Q = Some(Queue::u8());
    *RESP_Q = Some(Queue::u8());

    let (mut cmd_p, cmd_c) = CMD_Q.as_mut().unwrap().split();
    let (resp_p, resp_c) = RESP_Q.as_mut().unwrap().split();

    let mut serial = Serial::usart2(
      p.USART2,
      (tx, rx),
      Config::default().baudrate(115_200.bps()),
      clocks,
      &mut rcc.apb1r1,
    );

    serial.listen(Rxne);

    let at_parser = at::ATParser::new(serial, (cmd_c, resp_p));

    ctx.spawn.at_loop().unwrap();

    cmd_p.enqueue(Command::AT).unwrap();

    init::LateResources { at_parser }
  }

  #[idle]
  fn idle(_: idle::Context) -> ! {
    loop {
      wfi();
    }
  }

  #[task(schedule = [at_loop], resources = [at_parser])]
  fn at_loop(mut ctx: at_loop::Context) {
    ctx.resources.at_parser.lock(|at| at.spin());

    // Adjust this spin rate to set how often the request/response queue is checked
    ctx
      .schedule
      .at_loop(ctx.scheduled + 1_000_000.cycles())
      .unwrap();
  }

  #[task(binds = USART2, priority = 4, resources = [at_parser])]
  fn serial_irq(mut ctx: serial_irq::Context) {
    // ctx.resources.at_parser.lock(|at| at.handle_irq());
  }

  // spare interrupt used for scheduling software tasks
  extern "C" {
    fn UART5();
    fn LCD();
  }
};
