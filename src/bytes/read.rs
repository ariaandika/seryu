use core::{ops, slice};

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

    /// Returns the read bytes count.
    #[inline]
    pub const fn read_len(&self) -> usize {
        self.read
    }

    /// Returns the remaining bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Mark `count` bytes as read.
    ///
    /// Note that this will not mark past the remaining bytes.
    #[inline]
    pub fn read(&mut self, count: usize) {
        let count = self.bytes.len().min(count);
        self.read += count;
        self.bytes = unsafe { self.bytes.get_unchecked(count..) };
    }

    /// Reset the read bytes offset.
    #[inline]
    pub const fn reset(&mut self) {
        let len = self.read + self.bytes.len();
        self.bytes = unsafe { slice::from_raw_parts(self.bytes.as_ptr().sub(self.read), len) };
        self.read = 0;
    }
}

impl ops::Deref for Reader<'_> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.bytes
    }
}

impl<'a> From<&'a [u8]> for Reader<'a> {
    #[inline]
    fn from(value: &'a [u8]) -> Self {
        Self::new(value)
    }
}
