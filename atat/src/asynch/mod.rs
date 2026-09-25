mod client;
mod simple_client;

pub use client::Client;
use embedded_io_async::ErrorType;
pub use simple_client::SimpleClient;

use crate::{AtatCmd, Error};

pub trait AtatClient {
    type Writer: embedded_io_async::Write;

    /// Returns a mutable reference to the inner writer.
    fn inner(&mut self) -> &mut Self::Writer;

    /// Send an AT command with a custom write closure.
    async fn send_with<Cmd: AtatCmd>(
        &mut self,
        cmd: &Cmd,
        write: impl AsyncFnOnce(&mut Self::Writer) -> Result<(), <Self::Writer as ErrorType>::Error>,
    ) -> Result<Cmd::Response, Error>;

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

impl<T> AtatClient for &mut T
where
    T: AtatClient,
{
    type Writer = T::Writer;

    fn inner(&mut self) -> &mut T::Writer {
        T::inner(self)
    }

    async fn send_with<Cmd: AtatCmd>(
        &mut self,
        cmd: &Cmd,
        write: impl AsyncFnOnce(&mut T::Writer) -> Result<(), <Self::Writer as ErrorType>::Error>,
    ) -> Result<Cmd::Response, Error> {
        T::send_with(self, cmd, write).await
    }

    async fn send<Cmd: AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, Error> {
        T::send(self, cmd).await
    }
}
