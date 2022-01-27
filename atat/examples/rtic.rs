#![no_main]
#![no_std]
pub mod common;

#[rtic::app(device = stm32l4xx_hal::pac, peripherals = true, dispatchers = [UART5, LCD])]
mod app {
    use super::common;
    use bbqueue::BBBuffer;
    use stm32l4xx_hal::{
        pac::{Peripherals, USART2},
        prelude::*,
        serial::{Config, Event::Rxne, Rx, Serial},
        timer::Timer,
    };
    use systick_monotonic::*;

    use atat::{AtatClient, ClientBuilder, Clock, ComQueue, Queues};

    use fugit::ExtU32;
    use heapless::spsc::Queue;

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

        fn start(
            &mut self,
            _duration: fugit::TimerDurationU32<TIMER_HZ>,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn cancel(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn wait(&mut self) -> nb::Result<(), Self::Error> {
            Ok(())
        }
    }

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<100>; // 100 Hz / 10 ms granularity

    #[shared]
    struct SharedResources {
        ingress:
            atat::IngressManager<atat::DefaultDigester, atat::DefaultUrcMatcher, 256, 1024, 512>,
        rx: Rx<USART2>,
    }
    #[local]
    struct LocalResources {}

    #[init()]
    fn init(ctx: init::Context) -> (SharedResources, LocalResources, init::Monotonics()) {
        static mut RES_QUEUE: BBBuffer<1024> = BBBuffer::new();
        static mut URC_QUEUE: BBBuffer<512> = BBBuffer::new();
        static mut COM_QUEUE: ComQueue = Queue::new();

        let p = Peripherals::take().unwrap();

        let mut flash = p.FLASH.constrain();
        let mut rcc = p.RCC.constrain();
        let mut pwr = p.PWR.constrain(&mut rcc.apb1r1);

        let systick = ctx.core.SYST;
        // Initialize the monotonic
        let mono = Systick::new(systick, 16_000_000);

        let mut gpioa = p.GPIOA.split(&mut rcc.ahb2);
        // let mut gpiob = p.GPIOB.split(&mut rcc.ahb2);

        // clock configuration using the default settings (all clocks run at 8 MHz)
        let clocks = rcc.cfgr.freeze(&mut flash.acr, &mut pwr);
        // TRY this alternate clock configuration (clocks run at nearly the maximum frequency)
        // let clocks = rcc.cfgr.sysclk(64.mhz()).pclk1(32.mhz()).freeze(&mut flash.acr);

        // The Serial API is highly generic
        // TRY the commented out, different pin configurations
        // let tx = gpioa.pa9.into_alternate(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);
        // let tx = gpiob.pb6.into_alternate(&mut gpiob.moder, &mut gpiob.otyper, &mut gpiob.afrl);

        // let rx = gpioa.pa10.into_alternate(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);
        // let rx = gpiob.pb7.into_alternate(&mut gpiob.moder, &mut gpiob.otyper, &mut gpiob.afrl);

        let tx = gpioa
            .pa2
            .into_alternate(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);
        let rx = gpioa
            .pa3
            .into_alternate(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);

        let timer = Timer::tim7(p.TIM7, 1.hz(), clocks, &mut rcc.apb1r1);
        let at_clock: AtClock<_, 1> = AtClock::new(timer);

        let mut serial = Serial::usart2(
            p.USART2,
            (tx, rx),
            Config::default().baudrate(115_200.bps()),
            clocks,
            &mut rcc.apb1r1,
        );

        serial.listen(Rxne);

        let queues = Queues {
            res_queue: unsafe { RES_QUEUE.try_split_framed().unwrap() },
            urc_queue: unsafe { URC_QUEUE.try_split_framed().unwrap() },
            com_queue: unsafe { COM_QUEUE.split() },
        };

        let (tx, rx) = serial.split();
        let (mut client, ingress) =
            ClientBuilder::new(tx, at_clock, atat::Config::new(atat::Mode::Timeout)).build(queues);

        at_loop::spawn().ok();

        client.send(&common::AT).unwrap();

        (
            SharedResources { ingress, rx },
            LocalResources {},
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            rtic::export::wfi();
        }
    }

    #[task(shared = [ingress])]
    fn at_loop(mut ctx: at_loop::Context) {
        ctx.shared.ingress.lock(|at| at.digest());

        // Adjust this spin rate to set how often the request/response queue is checked
        at_loop::spawn_at(crate::app::monotonics::now() + 5u32.secs()).ok();
    }

    #[task(binds = USART2, priority = 4, shared = [ingress, rx])]
    fn serial_irq(ctx: serial_irq::Context) {
        let serial_irq::SharedResources {
            mut rx,
            mut ingress,
        } = ctx.shared;

        rx.lock(|rx| {
            if let Ok(d) = nb::block!(rx.read()) {
                ingress.lock(|ingress| ingress.write(&[d]));
            }
        });
    }
}

#[panic_handler] // panicking behavior
fn panic(_info: &core::panic::PanicInfo) -> ! {
    cortex_m::peripheral::SCB::sys_reset();
}
