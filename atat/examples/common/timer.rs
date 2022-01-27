use cortex_m::{interrupt, peripheral::DWT};

pub struct DwtTimer<const TIMER_HZ: u32> {
    end_time: Option<fugit::TimerInstantU32<TIMER_HZ>>,
}

impl<const TIMER_HZ: u32> DwtTimer<TIMER_HZ> {
    pub fn new() -> Self {
        Self { end_time: None }
    }

    pub fn now() -> u64 {
        static mut DWT_OVERFLOWS: u32 = 0;
        static mut OLD_DWT: u32 = 0;

        interrupt::free(|_| {
            // Safety: These static mut variables are accessed in an interrupt free section.
            let (overflows, last_cnt) = unsafe { (&mut DWT_OVERFLOWS, &mut OLD_DWT) };

            let cyccnt = DWT::cycle_count();

            if cyccnt <= *last_cnt {
                *overflows += 1;
            }

            let ticks = (*overflows as u64) << 32 | (cyccnt as u64);
            *last_cnt = cyccnt;

            ticks
        })
    }
}

impl<const TIMER_HZ: u32> Default for DwtTimer<TIMER_HZ> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const TIMER_HZ: u32> fugit_timer::Timer<TIMER_HZ> for DwtTimer<TIMER_HZ> {
    type Error = core::convert::Infallible;

    fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ> {
        fugit::TimerInstantU32::from_ticks(Self::now() as u32)
    }

    fn start(&mut self, duration: fugit::TimerDurationU32<TIMER_HZ>) -> Result<(), Self::Error> {
        let end = self.now() + duration;
        self.end_time.replace(end);
        Ok(())
    }

    fn cancel(&mut self) -> Result<(), Self::Error> {
        self.end_time.take();
        Ok(())
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        let now = self.now();
        match self.end_time {
            Some(end) if end <= now => Ok(()),
            _ => Err(nb::Error::WouldBlock),
        }
    }
}
