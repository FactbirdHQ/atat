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

    /// Wait for a response to become available
    pub async fn wait(&self) {
        self.1.wait().await
    }

    /// Get whether a response is available
    pub fn available(&self) -> bool {
        self.1.signaled()
    }

    /// Get a guard to response
    pub fn get<'a>(&'a self) -> ResponseSlotGuard<'a, N> {
        self.0.try_lock().unwrap()
    }

    pub(crate) fn signal_prompt(&self, prompt: u8) -> Result<(), SlotInUseError> {
        if self.1.signaled() {
            return Err(SlotInUseError);
        }
        let buf = self.0.try_lock().unwrap();
        let mut res = buf.borrow_mut();
        *res = Response::Prompt(prompt);
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
        let buf = self.0.try_lock().unwrap();
        let mut res = buf.borrow_mut();
        *res = response.into();
        self.1.signal(());
        Ok(())
    }
}
