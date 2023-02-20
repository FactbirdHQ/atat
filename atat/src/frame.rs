use crate::{InternalError, Response};
use bbqueue::framed::FrameProducer;
use bincode::{BorrowDecode, Encode};

#[derive(Debug, Clone, Copy, Encode, BorrowDecode, PartialEq)]
pub enum Frame<'a> {
    Response(&'a [u8]),
    Prompt(u8),
    ReadError,
    WriteError,
    TimeoutError,
    InvalidResponseError,
    AbortedError,
    OverflowError,
    ParseError,
    OtherError,
    CmeError(u16),
    CmsError(u16),
    ConnectionError(u8),
    CustomError(&'a [u8]),
}

const BINCODE_CONFIG: bincode::config::Configuration =
    bincode::config::standard().with_variable_int_encoding();

impl Frame<'_> {
    pub const fn max_len(&self) -> usize {
        // bincode enum discrimonator is 1 byte when variable_int_encoding is specified
        1 + match self {
            Frame::Response(b) => variable_int_encoding_length(b.len()) + b.len(),
            Frame::Prompt(p) => variable_int_encoding_length(*p as usize),
            Frame::CmeError(e) => variable_int_encoding_length(*e as usize),
            Frame::CmsError(e) => variable_int_encoding_length(*e as usize),
            Frame::CustomError(b) => variable_int_encoding_length(b.len()) + b.len(),
            _ => 0,
        }
    }

    pub fn encode(&self, buffer: &mut [u8]) -> usize {
        let encoded = bincode::encode_into_slice(self, buffer, BINCODE_CONFIG).unwrap();
        assert!(encoded <= self.max_len());
        encoded
    }
}

const fn variable_int_encoding_length(len: usize) -> usize {
    // See https://docs.rs/bincode/2.0.0-rc.2/bincode/config/struct.Configuration.html#method.with_variable_int_encoding
    if len < 251 {
        1
    } else {
        assert!(len < usize::pow(2, 16));
        1 + 2
    }
}

impl<'a> Frame<'a> {
    pub fn decode(buffer: &'a [u8]) -> Self {
        let (frame, decoded) = bincode::borrow_decode_from_slice(buffer, BINCODE_CONFIG).unwrap();
        assert_eq!(buffer.len(), decoded);
        frame
    }
}

impl<'a> From<Result<&'a [u8], InternalError<'a>>> for Frame<'a> {
    fn from(value: Result<&'a [u8], InternalError<'a>>) -> Self {
        match value {
            Ok(slice) => Frame::Response(slice),
            Err(error) => error.into(),
        }
    }
}

impl<'a> From<InternalError<'a>> for Frame<'a> {
    fn from(v: InternalError<'a>) -> Self {
        match v {
            InternalError::Read => Frame::ReadError,
            InternalError::Write => Frame::WriteError,
            InternalError::Timeout => Frame::TimeoutError,
            InternalError::InvalidResponse => Frame::InvalidResponseError,
            InternalError::Aborted => Frame::AbortedError,
            InternalError::Overflow => Frame::OverflowError,
            InternalError::Parse => Frame::ParseError,
            InternalError::Error => Frame::OtherError,
            InternalError::CmeError(e) => Frame::CmeError(e as u16),
            InternalError::CmsError(e) => Frame::CmsError(e as u16),
            InternalError::ConnectionError(e) => Frame::ConnectionError(e as u8),
            InternalError::Custom(e) => Frame::CustomError(e),
        }
    }
}

impl<'a> From<Frame<'a>> for Response<'a> {
    fn from(value: Frame<'a>) -> Self {
        match value {
            Frame::Response(slice) => Self::Result(Ok(slice)),
            Frame::Prompt(value) => Self::Prompt(value),
            Frame::ReadError => Self::Result(Err(InternalError::Read)),
            Frame::WriteError => Self::Result(Err(InternalError::Write)),
            Frame::TimeoutError => Self::Result(Err(InternalError::Timeout)),
            Frame::InvalidResponseError => Self::Result(Err(InternalError::InvalidResponse)),
            Frame::AbortedError => Self::Result(Err(InternalError::Aborted)),
            Frame::OverflowError => Self::Result(Err(InternalError::Overflow)),
            Frame::ParseError => Self::Result(Err(InternalError::Parse)),
            Frame::OtherError => Self::Result(Err(InternalError::Error)),
            Frame::CmeError(e) => Self::Result(Err(InternalError::CmeError(e.try_into().unwrap()))),
            Frame::CmsError(e) => Self::Result(Err(InternalError::CmsError(e.try_into().unwrap()))),
            Frame::ConnectionError(e) => {
                Self::Result(Err(InternalError::ConnectionError(e.try_into().unwrap())))
            }
            Frame::CustomError(e) => Self::Result(Err(InternalError::Custom(e))),
        }
    }
}

pub(crate) trait FrameProducerExt<'a> {
    fn enqueue(&mut self, frame: Frame<'a>) -> Result<(), ()>;
}

impl<const N: usize> FrameProducerExt<'_> for FrameProducer<'_, N> {
    fn enqueue(&mut self, frame: Frame<'_>) -> Result<(), ()> {
        if let Ok(mut grant) = self.grant(frame.max_len()) {
            let len = frame.encode(grant.as_mut());
            grant.commit(len);
            Ok(())
        } else {
            Err(())
        }
    }
}
