use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::Publisher};
use embedded_io::ErrorType;
use heapless::String;

pub struct TxMock<'a> {
    buf: String<64>,
    publisher: Publisher<'a, CriticalSectionRawMutex, String<64>, 1, 1, 1>,
}

#[derive(Debug)]
pub struct IoError;

impl embedded_io::Error for IoError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

impl<'a> TxMock<'a> {
    pub fn new(publisher: Publisher<'a, CriticalSectionRawMutex, String<64>, 1, 1, 1>) -> Self {
        TxMock {
            buf: String::new(),
            publisher,
        }
    }
}

impl ErrorType for TxMock<'_> {
    type Error = IoError;
}

impl embedded_io::Write for TxMock<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for c in buf {
            self.buf.push(*c as char).map_err(|_| IoError)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.publisher.try_publish(self.buf.clone()).unwrap();
        self.buf.clear();
        Ok(())
    }
}

impl embedded_io_async::Write for TxMock<'_> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for c in buf {
            self.buf.push(*c as char).map_err(|_| IoError)?;
        }
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.publisher.publish(self.buf.clone()).await;
        self.buf.clear();
        Ok(())
    }
}
