#![no_main]
#![no_std]
pub mod common;

use defmt_rtt as _;
use panic_probe as _; // global logger

#[cfg(feature = "defmt")]
defmt::timestamp!("{=u64}", {
    common::timer::DwtTimer::<80_000_000>::now() / 80_000
});

pub mod pac {
    pub const NVIC_PRIO_BITS: u8 = 2;
    pub use cortex_m_rt::interrupt;
    pub use embassy_stm32::pac::Interrupt as interrupt;
    pub use embassy_stm32::pac::*;
}

#[rtic::app(device = crate::pac, peripherals = true, dispatchers = [UART4, UART5])]
mod app {
    use super::common::{self, timer::DwtTimer, Urc};
    use bbqueue::BBBuffer;
    use dwt_systick_monotonic::*;
    use embedded_hal_nb::nb;

    use embassy_stm32::{dma::NoDma, gpio, peripherals::USART3};

    use atat::{AtatClient, ClientBuilder, Queues};

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
            atat::AtDigester<Urc>,
            RX_BUFFER_BYTES,
            RES_CAPACITY_BYTES,
            URC_CAPACITY_BYTES,
        >,
    }
    #[local]
    struct LocalResources {
        rx: embassy_stm32::usart::UartRx<'static, USART3>,
        client: atat::Client<
            embassy_stm32::usart::UartTx<'static, USART3>,
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
            res_queue: unsafe { RES_QUEUE.try_split_framed().unwrap() },
            urc_queue: unsafe { URC_QUEUE.try_split_framed().unwrap() },
        };

        let (client, ingress) = ClientBuilder::new(
            tx,
            DwtTimer::<80_000_000>::new(),
            atat::AtDigester::new(),
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
        #[cfg(feature = "defmt")]
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
            if let Ok(d) = nb::block!(rx.nb_read()) {
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
