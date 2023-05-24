use embassy_time::{Duration, Instant};

pub struct BlockingTimer {
    expires_at: Instant,
}

impl BlockingTimer {
    pub fn after(duration: Duration) -> Self {
        Self {
            expires_at: Instant::now() + duration,
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
