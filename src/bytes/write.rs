use core::mem::MaybeUninit;
use core::{ops, slice};

/// Bytes writing helper.
#[derive(Debug)]
pub struct Writer<'a> {
    bytes: &'a mut [MaybeUninit<u8>],
    write: usize,
}

impl<'a> Writer<'a> {
    /// Creates new [`Writer`].
    #[inline]
    pub const fn new(bytes: &'a mut [MaybeUninit<u8>]) -> Self {
        Self { bytes, write: 0 }
    }

    /// Returns total buffer capacity.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.bytes.len()
    }

    /// Returns the remaining buffer capacity.
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.bytes.len() - self.write
    }

    /// Returns the written bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.bytes.as_ptr().cast(), self.write) }
    }

    /// Returns the written bytes.
    #[inline]
    pub const fn as_mut_bytes(&mut self) -> &'a mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.bytes.as_mut_ptr().cast(), self.write) }
    }

    /// Returns the remaining unwritten bytes.
    #[inline]
    pub const fn remaining_mut(&mut self) -> &'a mut [MaybeUninit<u8>] {
        unsafe {
            slice::from_raw_parts_mut(
                self.bytes.as_mut_ptr().add(self.write),
                self.bytes.len() - self.write,
            )
        }
    }

    /// Set initialized bytes length.
    ///
    /// # Safety
    ///
    /// `new_len` of the buffer must be initialized.
    #[inline]
    pub const unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.bytes.len());
        self.write = new_len;
    }

    /// Write bytes from slice.
    #[inline]
    pub fn write(&mut self, bytes: &[u8]) -> usize {
        let c = self.remaining().min(bytes.len());
        unsafe {
            self.bytes
                .as_mut_ptr()
                .copy_from_nonoverlapping(bytes.as_ptr().cast(), c)
        };
        self.write += c;
        c
    }
}

impl<'a> ops::Deref for Writer<'a> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl<'a> ops::DerefMut for Writer<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_bytes()
    }
}
