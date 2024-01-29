mod client;

pub use client::Client;

use crate::{AtatCmd, Error};

pub trait AtatClient {
    /// Send an AT command.
    ///
    /// `cmd` must implement [`AtatCmd`].
    ///
    /// This function will also make sure that at least `self.config.cmd_cooldown`
    /// has passed since the last response or URC has been received, to allow
    /// the slave AT device time to deliver URC's.
    async fn send<Cmd: AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, Error>;

    async fn send_retry<Cmd: AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, Error> {
        for attempt in 1..=Cmd::ATTEMPTS {
            if attempt > 1 {
                debug!("Attempt {}:", attempt);
            }

            match self.send(cmd).await {
                Err(Error::Timeout) => {}
                Err(Error::Parse) => {
                    if !Cmd::REATTEMPT_ON_PARSE_ERR {
                        return Err(Error::Parse);
                    }
                }
                r => return r,
            }
        }
        Err(Error::Timeout)
    }
}
