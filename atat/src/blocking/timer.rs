use crate::error::Error;
use embassy_time::{Duration, Instant};

pub struct Timer {
    expires_at: Instant,
}

impl Timer {
    pub fn after(duration: Duration) -> Self {
        Self {
            expires_at: Instant::now() + duration,
        }
    }

    pub fn with_timeout<F, R>(timeout: Duration, mut e: F) -> Result<R, Error>
    where
        F: FnMut() -> Option<Result<R, Error>>,
    {
        let timer = Timer::after(timeout);

        loop {
            if let Some(res) = e() {
                return res;
            }
            if timer.expires_at <= Instant::now() {
                return Err(Error::Timeout);
            }
        }
    }

    pub fn wait(self) {
        loop {
            if self.expires_at <= Instant::now() {
                break;
            }
        }
    }
}
