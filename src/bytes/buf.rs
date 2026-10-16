/// Readable bytes.
pub trait Buf<'a> {
    /// Returns the remaining bytes.
    fn as_bytes(&self) -> &'a [u8];

    /// Assume `count` bytes has been consumed.
    fn advance(&mut self, count: usize);

    /// Read `len` of bytes.
    #[inline]
    fn read(&mut self, len: usize) -> Option<&'a [u8]> {
        let (lead, suff) = self.as_bytes().split_at_checked(len)?;
        self.advance(suff.len());
        Some(lead)
    }
}

impl<'a> Buf<'a> for &'a [u8] {
    #[inline]
    fn as_bytes(&self) -> &'a [u8] {
        self
    }

    #[inline]
    fn advance(&mut self, count: usize) {
        *self = &self[count..];
    }
}

impl<'a> Buf<'a> for &mut &'a [u8] {
    #[inline]
    fn as_bytes(&self) -> &'a [u8] {
        self
    }

    #[inline]
    fn advance(&mut self, count: usize) {
        **self = &self[count..];
    }
}
