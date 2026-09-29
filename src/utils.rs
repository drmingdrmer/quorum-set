/// Wrapper that allows formatting a slice of bytes as LowerHex
pub(crate) struct HexableSlice<'a>(&'a [u8]);

impl<'a, const N: usize> From<&'a [u8; N]> for HexableSlice<'a> {
    fn from(array: &'a [u8; N]) -> Self {
        Self(array.as_slice())
    }
}

impl<'a> std::fmt::LowerHex for HexableSlice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &byte in self.0 {
            write!(f, "{:0>2x}", byte)?;
        }
        Ok(())
    }
}
