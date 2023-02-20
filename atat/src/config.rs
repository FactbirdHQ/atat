use crate::blocking::Mode;

/// Configuration of both the ingress manager, and the AT client. Some of these
/// parameters can be changed on the fly, through issuing a [`Command`] from the
/// client.
///
/// [`Command`]: enum.Command.html
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Config {
    pub(crate) mode: Mode,
    pub(crate) cmd_cooldown: u32,
    pub(crate) tx_timeout: u32,
    pub(crate) flush_timeout: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Blocking,
            cmd_cooldown: 20,
            tx_timeout: 0,
            flush_timeout: 0,
        }
    }
}

impl Config {
    #[must_use]
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn tx_timeout(mut self, ms: u32) -> Self {
        self.tx_timeout = ms;
        self
    }

    #[must_use]
    pub const fn flush_timeout(mut self, ms: u32) -> Self {
        self.flush_timeout = ms;
        self
    }

    #[must_use]
    pub const fn cmd_cooldown(mut self, ms: u32) -> Self {
        self.cmd_cooldown = ms;
        self
    }
}
