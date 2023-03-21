use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};
use heapless::Vec;

use crate::{InternalError, Response};

pub type ResChannel<const INGRESS_BUF_SIZE: usize> =
    PubSubChannel<CriticalSectionRawMutex, ResMessage<INGRESS_BUF_SIZE>, 1, 1, 1>;

pub type ResPublisher<'sub, const INGRESS_BUF_SIZE: usize> =
    Publisher<'sub, CriticalSectionRawMutex, ResMessage<INGRESS_BUF_SIZE>, 1, 1, 1>;

pub type ResSubscription<'sub, const INGRESS_BUF_SIZE: usize> =
    Subscriber<'sub, CriticalSectionRawMutex, ResMessage<INGRESS_BUF_SIZE>, 1, 1, 1>;

#[derive(Debug, Clone, PartialEq)]
pub enum ResMessage<const N: usize> {
    Response(Vec<u8, N>),
    Prompt(u8),
    ReadError,
    WriteError,
    TimeoutError,
    InvalidResponseError,
    AbortedError,
    ParseError,
    OtherError,
    CmeError(u16),
    CmsError(u16),
    ConnectionError(u8),
    CustomError(Vec<u8, N>),
}

impl<const N: usize> ResMessage<N> {
    #[cfg(test)]
    pub(crate) fn empty_response() -> Self {
        ResMessage::Response(Vec::new())
    }

    pub fn response(value: &[u8]) -> Self {
        ResMessage::Response(Vec::from_slice(value).unwrap())
    }
}

impl<'a, const N: usize> From<Result<&'a [u8], InternalError<'a>>> for ResMessage<N> {
    fn from(value: Result<&'a [u8], InternalError<'a>>) -> Self {
        match value {
            Ok(slice) => ResMessage::Response(Vec::from_slice(slice).unwrap()),
            Err(error) => error.into(),
        }
    }
}

impl<'a, const N: usize> From<InternalError<'a>> for ResMessage<N> {
    fn from(v: InternalError<'a>) -> Self {
        match v {
            InternalError::Read => ResMessage::ReadError,
            InternalError::Write => ResMessage::WriteError,
            InternalError::Timeout => ResMessage::TimeoutError,
            InternalError::InvalidResponse => ResMessage::InvalidResponseError,
            InternalError::Aborted => ResMessage::AbortedError,
            InternalError::Parse => ResMessage::ParseError,
            InternalError::Error => ResMessage::OtherError,
            InternalError::CmeError(e) => ResMessage::CmeError(e as u16),
            InternalError::CmsError(e) => ResMessage::CmsError(e as u16),
            InternalError::ConnectionError(e) => ResMessage::ConnectionError(e as u8),
            InternalError::Custom(e) => ResMessage::CustomError(Vec::from_slice(e).unwrap()),
        }
    }
}

impl<'a, const N: usize> From<&'a ResMessage<N>> for Response<'a> {
    fn from(value: &'a ResMessage<N>) -> Self {
        match value {
            ResMessage::Response(slice) => Self::Result(Ok(slice)),
            ResMessage::Prompt(value) => Self::Prompt(*value),
            ResMessage::ReadError => Self::Result(Err(InternalError::Read)),
            ResMessage::WriteError => Self::Result(Err(InternalError::Write)),
            ResMessage::TimeoutError => Self::Result(Err(InternalError::Timeout)),
            ResMessage::InvalidResponseError => Self::Result(Err(InternalError::InvalidResponse)),
            ResMessage::AbortedError => Self::Result(Err(InternalError::Aborted)),
            ResMessage::ParseError => Self::Result(Err(InternalError::Parse)),
            ResMessage::OtherError => Self::Result(Err(InternalError::Error)),
            ResMessage::CmeError(e) => {
                Self::Result(Err(InternalError::CmeError((*e).try_into().unwrap())))
            }
            ResMessage::CmsError(e) => {
                Self::Result(Err(InternalError::CmsError((*e).try_into().unwrap())))
            }
            ResMessage::ConnectionError(e) => Self::Result(Err(InternalError::ConnectionError(
                (*e).try_into().unwrap(),
            ))),
            ResMessage::CustomError(e) => Self::Result(Err(InternalError::Custom(e))),
        }
    }
}
