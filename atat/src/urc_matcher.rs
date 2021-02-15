use heapless::{ArrayLength, Vec};

/// A user-defined URC matcher
///
/// This is used to detect and consume URCs that are not terminated with
/// standard response codes like "OK". An example could be an URC that returns
/// length-value (LV) encoded data without a terminator.
///
/// Note that you should only detect and consume but not process URCs.
/// Processing should be done by an [`AtatUrc`](trait.AtatUrc.html)
/// implementation.
///
/// A very simplistic example that can only handle the URC `+FOO,xx` (with
/// `xx` being two arbitrary characters) followed by CRLF:
///
/// ```
/// use atat::{UrcMatcher, UrcMatcherResult};
/// use heapless::{consts, ArrayLength, Vec};
///
/// struct FooUrcMatcher {}
///
/// impl UrcMatcher for FooUrcMatcher {
///     fn process<L: ArrayLength<u8>>(&mut self, buf: &mut Vec<u8, L>) -> UrcMatcherResult<L> {
///         if buf.starts_with(b"+FOO,") {
///             if buf.len() >= 9 {
///                 if &buf[7..9] == b"\r\n" {
///                     // URC is complete
///                     let data = Vec::from_slice(&buf[..9]).unwrap();
///                     *buf = Vec::from_slice(&buf[9..]).unwrap();
///                     UrcMatcherResult::Complete(data)
///                 } else {
///                     // Invalid, reject
///                     UrcMatcherResult::NotHandled
///                 }
///             } else {
///                 // Insufficient data
///                 UrcMatcherResult::Incomplete
///             }
///         } else {
///             UrcMatcherResult::NotHandled
///         }
///     }
/// }
/// ```
pub trait UrcMatcher {
    /// Take a look at `buf`. Then:
    ///
    /// - If the buffer contains a full URC, remove these bytes from the buffer
    ///   and return [`Complete`] with the matched data.
    /// - If it contains an incomplete URC, return [`Incomplete`].
    /// - Otherwise, return [`NotHandled`].
    ///
    /// [`Complete`]: enum.UrcMatcherResult.html#variant.Complete
    /// [`Incomplete`]: enum.UrcMatcherResult.html#variant.Incomplete
    /// [`NotHandled`]: enum.UrcMatcherResult.html#variant.NotHandled
    fn process<L: ArrayLength<u8>>(&mut self, buf: &mut Vec<u8, L>) -> UrcMatcherResult<L>;
}

/// The type returned from a custom URC matcher.
pub enum UrcMatcherResult<L: ArrayLength<u8>> {
    NotHandled,
    Incomplete,
    Complete(Vec<u8, L>),
}

/// A URC matcher that does nothing (it always returns [`NotHandled`][nothandled]).
///
/// [nothandled]: enum.UrcMatcherResult.html#variant.NotHandled
#[derive(Debug, Default)]
pub struct DefaultUrcMatcher;

impl UrcMatcher for DefaultUrcMatcher {
    fn process<L: ArrayLength<u8>>(&mut self, _: &mut Vec<u8, L>) -> UrcMatcherResult<L> {
        UrcMatcherResult::NotHandled
    }
}
