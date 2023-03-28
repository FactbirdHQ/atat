use crate::InternalError;
use heapless::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum Response<const N: usize> {
    Ok(Vec<u8, N>),
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

impl<const N: usize> Response<N> {
    pub fn ok(value: &[u8]) -> Self {
        Response::Ok(Vec::from_slice(value).unwrap())
    }
}

impl<const N: usize> Default for Response<N> {
    fn default() -> Self {
        Response::Ok(Vec::new())
    }
}

impl<'a, const N: usize> From<Result<&'a [u8], InternalError<'a>>> for Response<N> {
    fn from(value: Result<&'a [u8], InternalError<'a>>) -> Self {
        match value {
            Ok(slice) => Response::Ok(Vec::from_slice(slice).unwrap()),
            Err(error) => error.into(),
        }
    }
}

impl<'a, const N: usize> From<InternalError<'a>> for Response<N> {
    fn from(v: InternalError<'a>) -> Self {
        match v {
            InternalError::Read => Response::ReadError,
            InternalError::Write => Response::WriteError,
            InternalError::Timeout => Response::TimeoutError,
            InternalError::InvalidResponse => Response::InvalidResponseError,
            InternalError::Aborted => Response::AbortedError,
            InternalError::Parse => Response::ParseError,
            InternalError::Error => Response::OtherError,
            InternalError::CmeError(e) => Response::CmeError(e as u16),
            InternalError::CmsError(e) => Response::CmsError(e as u16),
            InternalError::ConnectionError(e) => Response::ConnectionError(e as u8),
            InternalError::Custom(e) => Response::CustomError(Vec::from_slice(e).unwrap()),
        }
    }
}

impl<'a, const N: usize> From<&'a Response<N>> for Result<&'a [u8], InternalError<'a>> {
    fn from(value: &'a Response<N>) -> Self {
        match value {
            Response::Ok(slice) => Ok(slice),
            Response::Prompt(_) => Ok(&[]),
            Response::ReadError => Err(InternalError::Read),
            Response::WriteError => Err(InternalError::Write),
            Response::TimeoutError => Err(InternalError::Timeout),
            Response::InvalidResponseError => Err(InternalError::InvalidResponse),
            Response::AbortedError => Err(InternalError::Aborted),
            Response::ParseError => Err(InternalError::Parse),
            Response::OtherError => Err(InternalError::Error),
            Response::CmeError(e) => Err(InternalError::CmeError((*e).try_into().unwrap())),
            Response::CmsError(e) => Err(InternalError::CmsError((*e).try_into().unwrap())),
            Response::ConnectionError(e) => {
                Err(InternalError::ConnectionError((*e).try_into().unwrap()))
            }
            Response::CustomError(e) => Err(InternalError::Custom(e)),
        }
    }
}
