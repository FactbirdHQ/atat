mod client;

pub use client::{Client, Mode};
use embedded_hal_nb::nb;

use crate::{AtatCmd, AtatUrc, Error};

pub trait AtatClient {
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
    fn send<A: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &A,
    ) -> nb::Result<A::Response, Error>;

    fn send_retry<A: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &A,
    ) -> nb::Result<A::Response, Error> {
        for attempt in 1..=A::ATTEMPTS {
            if attempt > 1 {
                debug!("Attempt {}:", attempt);
            }

            match self.send(cmd) {
                Err(nb::Error::Other(Error::Timeout)) => {}
                r => return r,
            }
        }
        Err(nb::Error::Other(Error::Timeout))
    }

    /// Checks if there are any URC's (Unsolicited Response Code) in
    /// queue from the ingress manager.
    ///
    /// Example:
    /// ```
    /// use atat::atat_derive::{AtatResp, AtatUrc};
    ///
    /// #[derive(Clone, AtatResp)]
    /// pub struct MessageWaitingIndication {
    ///     #[at_arg(position = 0)]
    ///     pub status: u8,
    ///     #[at_arg(position = 1)]
    ///     pub code: u8,
    /// }
    ///
    /// #[derive(Clone, AtatUrc)]
    /// pub enum Urc {
    ///     #[at_urc("+UMWI")]
    ///     MessageWaitingIndication(MessageWaitingIndication),
    /// }
    ///
    /// // match client.check_urc::<Urc>() {
    /// //     Some(Urc::MessageWaitingIndication(MessageWaitingIndication { status, code })) => {
    /// //         // Do something to act on `+UMWI` URC
    /// //     }
    /// // }
    /// ```
    fn check_urc<URC: AtatUrc>(&mut self) -> Option<URC::Response> {
        let mut return_urc = None;
        self.peek_urc_with::<URC, _>(|urc| {
            return_urc = Some(urc);
            true
        });
        return_urc
    }

    fn peek_urc_with<URC: AtatUrc, F: FnOnce(URC::Response) -> bool>(&mut self, f: F);

    /// Check if there are any responses enqueued from the ingress manager.
    ///
    /// The function will return `nb::Error::WouldBlock` until a response or an
    /// error is available, or a timeout occurs and `config.mode` is Timeout.
    ///
    /// This function is usually only called through [`send`].
    ///
    /// [`send`]: #method.send
    fn check_response<A: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &A,
    ) -> nb::Result<A::Response, Error>;

    /// Get the configured mode of the client.
    ///
    /// Options are:
    /// - `NonBlocking`
    /// - `Blocking`
    /// - `Timeout`
    fn get_mode(&self) -> Mode;

    /// Reset the client, queues and ingress buffer, discarding any contents
    fn reset(&mut self);
}
