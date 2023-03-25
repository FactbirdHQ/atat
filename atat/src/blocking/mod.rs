mod client;
mod timer;

pub use client::Client;

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
    fn send<A: AtatCmd<LEN>, const LEN: usize>(&mut self, cmd: &A) -> Result<A::Response, Error>;

    fn send_retry<A: AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: &A,
    ) -> Result<A::Response, Error> {
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
    fn try_read_urc<Urc: AtatUrc>(&mut self) -> Option<Urc::Response> {
        let mut first = None;
        self.try_read_urc_with::<Urc, _>(|urc, _| {
            first = Some(urc);
            true
        });
        first
    }

    fn try_read_urc_with<Urc: AtatUrc, F: for<'b> FnOnce(Urc::Response, &'b [u8]) -> bool>(
        &mut self,
        handle: F,
    ) -> bool;

    fn max_urc_len() -> usize;
}
