mod client;
mod simple_client;

pub use client::Client;
use embedded_io::ErrorType;
pub use simple_client::SimpleClient;

use crate::{AtatCmd, Error};

pub trait AtatClient {
    type Writer: embedded_io::Write;

    /// Returns a mutable reference to the inner writer.
    fn inner(&mut self) -> &mut Self::Writer;

    /// Send an AT command with a custom write closure.
    fn send_with<Cmd: AtatCmd>(
        &mut self,
        cmd: &Cmd,
        write: impl FnOnce(&mut Self::Writer) -> Result<(), <Self::Writer as ErrorType>::Error>,
    ) -> Result<Cmd::Response, Error>;

    /// Send an AT command.
    ///
    /// `cmd` must implement [`AtatCmd`].
    ///
    /// This function will block until a response is received, if in Timeout or
    /// Blocking mode. In Nonblocking mode, the send can be called until it no
    /// longer returns `nb::Error::WouldBlock`, or `self.check_response(cmd)` can
    /// be called, with the same result.
    ///
    /// This function will also make sure that at least `self.config.cmd_cooldown`
    /// has passed since the last response or URC has been received, to allow
    /// the slave AT device time to deliver URC's.
    fn send<A: AtatCmd>(&mut self, cmd: &A) -> Result<A::Response, Error>;

    fn send_retry<A: AtatCmd>(&mut self, cmd: &A) -> Result<A::Response, Error> {
        for attempt in 1..=A::ATTEMPTS {
            if attempt > 1 {
                debug!("Attempt {}:", attempt);
            }

            match self.send(cmd) {
                Err(Error::Timeout) => {}
                r => return r,
            }
        }
        Err(Error::Timeout)
    }
}
