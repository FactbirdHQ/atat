use core::cell::RefCell;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    mutex::{Mutex, MutexGuard},
    signal::Signal,
};
use heapless::Vec;

use crate::{InternalError, Response};

pub struct ResponseSlot<const N: usize>(
    Mutex<CriticalSectionRawMutex, RefCell<Response<N>>>,
    Signal<CriticalSectionRawMutex, ()>,
);

pub type ResponseSlotGuard<'a, const N: usize> =
    MutexGuard<'a, CriticalSectionRawMutex, RefCell<Response<N>>>;

#[derive(Debug)]
pub struct SlotInUseError;

impl<const N: usize> ResponseSlot<N> {
    pub const fn new() -> Self {
        Self(
            Mutex::new(RefCell::new(Response::Ok(Vec::new()))),
            Signal::new(),
        )
    }

    /// Reset the current response slot
    pub fn reset(&self) {
        self.1.reset();
    }

    /// Wait for a response to be signaled and get a guard to the response
    pub async fn get<'a>(&'a self) -> ResponseSlotGuard<'a, N> {
        self.1.wait().await;

        // The mutex is not locked when signal is emitted
        self.0.try_lock().unwrap()
    }

    /// If signaled, get a guard to the response
    pub fn try_get<'a>(&'a self) -> Option<ResponseSlotGuard<'a, N>> {
        if self.1.signaled() {
            // The mutex is not locked when signal is emitted
            Some(self.0.try_lock().unwrap())
        } else {
            None
        }
    }

    pub(crate) fn signal_prompt(&self, prompt: u8) -> Result<(), SlotInUseError> {
        if self.1.signaled() {
            return Err(SlotInUseError);
        }

        // Not currently signaled: We know that the client is not currently holding the response slot guard
        {
            let buf = self.0.try_lock().unwrap();
            let mut res = buf.borrow_mut();
            *res = Response::Prompt(prompt);
        }

        // Mutex is unlocked before we signal
        self.1.signal(());
        Ok(())
    }

    pub(crate) fn signal_response(
        &self,
        response: Result<&[u8], InternalError>,
    ) -> Result<(), SlotInUseError> {
        if self.1.signaled() {
            return Err(SlotInUseError);
        }

        // Not currently signaled: We know that the client is not currently holding the response slot guard
        {
            let buf = self.0.try_lock().unwrap();
            let mut res = buf.borrow_mut();
            *res = response.into();
        }

        // Mutex is unlocked before we signal
        self.1.signal(());
        Ok(())
    }
}
