#![no_main]
#![no_std]

use stm32l4xx_hal as hal;

mod common;

use hal::{
    pac::{Peripherals, USART2},
    prelude::*,
    serial::{Config, Event::Rxne, Rx, Serial},
    timer::Timer,
};

use atat::{
    digest::DefaultDigester, urc_matcher::DefaultUrcMatcher, ClientBuilder, ComQueue, Queues,
    ResQueue, UrcQueue,
};
use rtic::{app, export::wfi};

use heapless::spsc::Queue;

#[app(device = hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        ingress: atat::IngressManager<atat::DefaultDigester, atat::DefaultUrcMatcher, 256, 10>,
        rx: Rx<USART2>,
    }

    #[init(spawn = [at_loop])]
    fn init(ctx: init::Context) -> init::LateResources {
        static mut RES_QUEUE: ResQueue<256> = Queue(heapless::i::Queue::u8());
        static mut URC_QUEUE: UrcQueue<256, 10> = Queue(heapless::i::Queue::u8());
        static mut COM_QUEUE: ComQueue = Queue(heapless::i::Queue::u8());

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

        let mut serial = Serial::usart2(
            p.USART2,
            (tx, rx),
            Config::default().baudrate(115_200.bps()),
            clocks,
            &mut rcc.apb1r1,
        );

        serial.listen(Rxne);

        let queues = Queues {
            res_queue: unsafe { RES_QUEUE.split() },
            urc_queue: unsafe { URC_QUEUE.split() },
            com_queue: unsafe { COM_QUEUE.split() },
        };

        let (tx, rx) = serial.split();
        let (mut client, ingress) =
            ClientBuilder::new(tx, timer, atat::Config::new(atat::Mode::Timeout)).build(queues);

        ctx.spawn.at_loop().unwrap();

        let response = client.send(&common::AT).unwrap();

        init::LateResources { ingress, rx }
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            wfi();
        }
    }

    #[task(spawn = [at_loop], resources = [ingress])]
    fn at_loop(mut ctx: at_loop::Context) {
        ctx.resources.ingress.lock(|at| at.digest());

        // Adjust this spin rate to set how often the request/response queue is checked
        ctx.spawn
            .at_loop()
            // .at_loop(ctx.scheduled + 1_000_000.cycles())
            .unwrap();
    }

    #[task(binds = USART2, priority = 4, resources = [ingress, rx])]
    fn serial_irq(mut ctx: serial_irq::Context) {
        let rx = ctx.resources.rx;
        if let Ok(d) = nb::block!(rx.read()) {
            ctx.resources.ingress.write(&[d]);
        }
    }

    // spare interrupt used for scheduling software tasks
    extern "C" {
        fn UART5();
        fn LCD();
    }
};
