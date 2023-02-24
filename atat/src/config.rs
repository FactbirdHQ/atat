use embassy_time::Duration;

/// Configuration of both the ingress manager, and the AT client. Some of these
/// parameters can be changed on the fly, through issuing a [`Command`] from the
/// client.
///
/// [`Command`]: enum.Command.html
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Config {
    pub(crate) cmd_cooldown: Duration,
    pub(crate) tx_timeout: Duration,
    pub(crate) flush_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cmd_cooldown: Duration::from_millis(20),
            tx_timeout: Duration::from_ticks(0),
            flush_timeout: Duration::from_ticks(0),
        }
    }
}

impl Config {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn tx_timeout(mut self, duration: Duration) -> Self {
        self.tx_timeout = duration;
        self
    }

    #[must_use]
    pub const fn flush_timeout(mut self, duration: Duration) -> Self {
        self.flush_timeout = duration;
        self
    }

    #[must_use]
    pub const fn cmd_cooldown(mut self, duration: Duration) -> Self {
        self.cmd_cooldown = duration;
        self
    }
}
