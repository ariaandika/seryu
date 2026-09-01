use core::mem::MaybeUninit;
use core::slice;

use crate::bytes::InsufficientBuffer;

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

    /// Returns the remaining buffer capacity.
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.bytes.len()
    }

    /// Returns the bytes length that have been written.
    #[inline]
    pub const fn write_len(&self) -> usize {
        self.write
    }

    /// Returns the initialized bytes.
    #[inline]
    pub const fn init(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.bytes.as_ptr().sub(self.write).cast(), self.write) }
    }

    /// Returns the initialized bytes.
    #[inline]
    pub const fn init_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self.bytes.as_mut_ptr().sub(self.write).cast(), self.write)
        }
    }

    /// Returns the remaining uninitialized bytes.
    #[inline]
    pub const fn uninit_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self.bytes
    }
}

impl<'a> Writer<'a> {
    /// Clear the initialized bytes.
    ///
    /// The remaining capacity will be restored to the initial buffer size.
    #[inline]
    pub const fn clear(&mut self) {
        self.bytes = unsafe {
            slice::from_raw_parts_mut(
                self.bytes.as_mut_ptr().sub(self.write),
                self.bytes.len() + self.write,
            )
        };
        self.write = 0;
    }

    /// Assume the first `count` of bytes is initialized.
    ///
    /// The uninitialized buffer can be retrieved using [`Writer::uninit_mut`].
    ///
    /// # Safety
    ///
    /// The fist `count` of bytes must be initialized.
    #[inline]
    pub const unsafe fn assume_init_len(&mut self, count: usize) {
        debug_assert!(count <= self.bytes.len());
        self.bytes = unsafe {
            slice::from_raw_parts_mut(self.bytes.as_mut_ptr().add(count), self.bytes.len() - count)
        };
        self.write += count;
    }

    /// # Safety
    ///
    /// `len <= self.remaining()`
    #[inline]
    const unsafe fn write_inner(&mut self, ptr: *const u8, len: usize) {
        debug_assert!(len <= self.remaining());
        unsafe {
            self.bytes
                .as_mut_ptr()
                .copy_from_nonoverlapping(ptr.cast(), len);
            self.assume_init_len(len);
        };
    }
}

impl<'a> Writer<'a> {
    /// Write bytes from slice that skips bounds checking.
    ///
    /// # Safety
    ///
    /// The buffer must have enough remaining capacity to contains the bytes.
    ///
    /// Or in other words: `bytes.len() <= self.remaining()`.
    #[inline]
    pub const unsafe fn write_unchecked(&mut self, bytes: &[u8]) {
        debug_assert!(bytes.len() <= self.remaining());
        // SAFETY: the caller safety guarantee
        unsafe { self.write_inner(bytes.as_ptr(), bytes.len()) };
    }

    /// Write bytes from slice.
    ///
    /// Returns an error if the remaining capacity is not enough to contains the bytes.
    #[inline]
    pub const fn write(&mut self, bytes: &[u8]) -> Result<(), InsufficientBuffer> {
        if bytes.len() > self.remaining() {
            return Err(InsufficientBuffer);
        }
        // SAFETY: `bytes.len() <= self.remaining()`
        unsafe { self.write_unchecked(bytes) };
        Ok(())
    }

    /// Write bytes from slice.
    ///
    /// Returns the length of written bytes.
    ///
    /// Truncate the bytes if the remaining capacity is not enough to contains the bytes.
    #[inline]
    pub fn write_truncated(&mut self, bytes: &[u8]) -> usize {
        let len = self.remaining().min(bytes.len());
        // SAFETY: `len <= self.remaining()`
        unsafe { self.write_inner(bytes.as_ptr(), len) };
        len
    }
}
