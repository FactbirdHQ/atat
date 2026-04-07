use embassy_time::{Duration, Instant};

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
    pub(crate) get_response_timeout: GetTimeout,
}

pub type GetTimeout = fn(Instant, Duration) -> Instant;

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

fn get_response_timeout(start: Instant, duration: Duration) -> Instant {
    start + duration
}

impl Config {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cmd_cooldown: Duration::from_millis(20),
            tx_timeout: Duration::from_millis(1000),
            flush_timeout: Duration::from_millis(1000),
            get_response_timeout,
        }
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

    /// Set a custom computation for determining the reponse timeout instant
    /// for a request sent at a specific time. The timeout is recomputed
    /// continously, so it is possible to for example artificially extend the
    /// timeout if for example flow control has hindered the device to actually
    /// communicate during the period from the request is sent until the
    /// response is expected.
    #[must_use]
    pub const fn get_response_timeout(mut self, compute: GetTimeout) -> Self {
        self.get_response_timeout = compute;
        self
    }
}
