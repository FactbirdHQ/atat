/// Wrapper for a byte-slice that formats it as a string if possible and as
/// bytes otherwise.
pub struct LossyStr<'a>(pub &'a [u8]);

impl<'a> core::fmt::Debug for LossyStr<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match core::str::from_utf8(self.0) {
            Ok(s) => write!(f, "{s:?}"),
            Err(_) => write!(f, "{:?}", self.0),
        }
    }
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for LossyStr<'a> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{=[u8]:a}", self.0)
    }
}
