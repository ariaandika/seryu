use core::slice;

/// Bytes reading helper.
///
/// This can be used in bytes reading where the size is unknown until the delimiter found.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    read: usize,
}

impl<'a> Reader<'a> {
    /// Creates new [`Reader`].
    #[inline]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, read: 0 }
    }

    /// Returns the remaining bytes length.
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.bytes.len()
    }

    /// Returns `true` if there is still remaining bytes.
    #[inline]
    pub const fn has_remaining(&self) -> bool {
        self.remaining() != 0
    }

    /// Returns the remaining bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Returns the bytes length that have been read.
    #[inline]
    pub const fn read_len(&self) -> usize {
        self.read
    }
}

impl<'a> Reader<'a> {
    /// Reset the read bytes offset.
    ///
    /// This will set reader to contains the initial bytes.
    #[inline]
    pub const fn reset(&mut self) {
        self.bytes = unsafe {
            slice::from_raw_parts(self.bytes.as_ptr().sub(self.read), self.read + self.remaining())
        };
        self.read = 0;
    }

    /// Assume `count` bytes has been read.
    #[inline]
    pub fn assume_read_len(&mut self, count: usize) {
        let count = self.remaining().min(count);
        self.bytes = unsafe {
            slice::from_raw_parts(self.bytes.as_ptr().add(count), self.remaining() - count)
        };
        self.read += count;
    }

    /// Read `N` chunk of bytes.
    ///
    /// Returns `None` if the remaining bytes is less than chunk length.
    #[inline]
    pub const fn read_chunk<const N: usize>(&mut self) -> Option<&'a [u8; N]> {
        let Some(remain) = self.bytes.len().checked_sub(N) else {
            return None;
        };
        let base = self.bytes.as_ptr();
        self.bytes = unsafe { slice::from_raw_parts(base.add(N), remain) };
        unsafe { Some(base.cast::<[u8; N]>().as_ref_unchecked()) }
    }
}
