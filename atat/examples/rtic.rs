#![no_main]
#![no_std]
pub mod common;

use defmt_rtt as _;
use panic_probe as _; // global logger

defmt::timestamp!("{=u64}", {
    common::timer::DwtTimer::<80_000_000>::now() / 80_000
});

#[rtic::app(device = stm32l4xx_hal::pac, peripherals = true, dispatchers = [UART5, LCD])]
mod app {
    use super::common::{self, timer::DwtTimer};
    use bbqueue::BBBuffer;
    use dwt_systick_monotonic::*;
    use stm32l4xx_hal::{
        pac::USART3,
        prelude::*,
        rcc::{ClockSecuritySystem, CrystalBypass, MsiFreq, PllConfig, PllDivider, PllSource},
        serial::{Config, Event::Rxne, Rx, Serial, Tx},
    };

    use atat::{AtatClient, ClientBuilder, ComQueue, Queues};

    use heapless::spsc::Queue;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<80_000_000>;

    const RX_BUFFER_BYTES: usize = 512;
    // Response queue is capable of holding one full RX buffer
    const RES_CAPACITY_BYTES: usize = RX_BUFFER_BYTES;
    // URC queue is capable of holding up to three full RX buffer
    const URC_CAPACITY_BYTES: usize = RX_BUFFER_BYTES * 3;

    #[shared]
    struct SharedResources {
        ingress: atat::IngressManager<
            atat::DefaultDigester,
            atat::DefaultUrcMatcher,
            RX_BUFFER_BYTES,
            RES_CAPACITY_BYTES,
            URC_CAPACITY_BYTES,
        >,
    }
    #[local]
    struct LocalResources {
        rx: Rx<USART3>,
        client: atat::Client<
            Tx<USART3>,
            DwtTimer<80_000_000>,
            80_000_000,
            RES_CAPACITY_BYTES,
            URC_CAPACITY_BYTES,
        >,
    }

    #[init()]
    fn init(mut ctx: init::Context) -> (SharedResources, LocalResources, init::Monotonics()) {
        // Create static queues for ATAT
        static mut RES_QUEUE: BBBuffer<RES_CAPACITY_BYTES> = BBBuffer::new();
        static mut URC_QUEUE: BBBuffer<URC_CAPACITY_BYTES> = BBBuffer::new();
        static mut COM_QUEUE: ComQueue = Queue::new();

        // Setup clocks & peripherals
        let mut flash = ctx.device.FLASH.constrain();
        let mut rcc = ctx.device.RCC.constrain();
        let mut pwr = ctx.device.PWR.constrain(&mut rcc.apb1r1);

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

        let mut gpioa = ctx.device.GPIOA.split(&mut rcc.ahb2);
        let mut gpiob = ctx.device.GPIOB.split(&mut rcc.ahb2);
        let mut gpiod = ctx.device.GPIOD.split(&mut rcc.ahb2);

        let mut wifi_nrst = gpiod
            .pd13
            .into_open_drain_output(&mut gpiod.moder, &mut gpiod.otyper);
        wifi_nrst.set_high();

        let tx = gpiod.pd8.into_alternate_push_pull(
            &mut gpiod.moder,
            &mut gpiod.otyper,
            &mut gpiod.afrh,
        );
        let rx = gpiod.pd9.into_alternate_push_pull(
            &mut gpiod.moder,
            &mut gpiod.otyper,
            &mut gpiod.afrh,
        );
        let rts = gpiob.pb1.into_alternate_push_pull(
            &mut gpiob.moder,
            &mut gpiob.otyper,
            &mut gpiob.afrl,
        );
        let cts = gpioa.pa6.into_alternate_push_pull(
            &mut gpioa.moder,
            &mut gpioa.otyper,
            &mut gpioa.afrl,
        );

        // Configure UART peripheral
        let mut serial = Serial::usart3(
            ctx.device.USART3,
            (tx, rx, rts, cts),
            Config::default().baudrate(115_200.bps()),
            clocks,
            &mut rcc.apb1r1,
        );
        serial.listen(Rxne);
        let (tx, rx) = serial.split();

        // Instantiate ATAT client & IngressManager
        let queues = Queues {
            res_queue: unsafe { RES_QUEUE.try_split_framed().unwrap() },
            urc_queue: unsafe { URC_QUEUE.try_split_framed().unwrap() },
            com_queue: unsafe { COM_QUEUE.split() },
        };

        let (client, ingress) = ClientBuilder::new(
            tx,
            DwtTimer::<80_000_000>::new(),
            atat::Config::new(atat::Mode::Timeout),
        )
        .build(queues);

        at_loop::spawn().ok();
        at_send::spawn(0).ok();

        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, 80_000_000);

        (
            SharedResources { ingress },
            LocalResources { client, rx },
            // Initialize the monotonic
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(local = [client], priority = 2)]
    fn at_send(ctx: at_send::Context, state: u8) {
        defmt::debug!("\r\n\r\n\r\n");

        match state {
            0 => {
                ctx.local
                    .client
                    .send(&common::general::GetManufacturerId)
                    .ok();
            }
            1 => {
                ctx.local.client.send(&common::general::GetModelId).ok();
            }
            2 => {
                ctx.local
                    .client
                    .send(&common::general::GetSoftwareVersion)
                    .ok();
            }
            3 => {
                ctx.local.client.send(&common::general::GetWifiMac).ok();
            }
            _ => cortex_m::asm::bkpt(),
        }
        // Adjust this spin rate to set how often the request/response queue is checked
        at_send::spawn_at(monotonics::now() + 1.secs(), state + 1).ok();
    }

    #[task(shared = [ingress], priority = 3)]
    fn at_loop(mut ctx: at_loop::Context) {
        ctx.shared.ingress.lock(|at| at.digest());

        // Adjust this spin rate to set how often the request/response queue is checked
        at_loop::spawn_at(monotonics::now() + 10.millis()).ok();
    }

    #[task(binds = USART3, priority = 4, shared = [ingress], local = [rx])]
    fn serial_irq(mut ctx: serial_irq::Context) {
        let rx = ctx.local.rx;
        ctx.shared.ingress.lock(|ingress| {
            if let Ok(d) = nb::block!(rx.read()) {
                ingress.write(&[d]);
            }
        });
    }
}

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
