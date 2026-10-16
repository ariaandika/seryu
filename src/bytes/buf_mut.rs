use core::mem::{self, MaybeUninit};

use crate::alloc::OutOfCapacity;

/// Writable bytes buffer.
pub trait BufMut {
    /// Returns the remaining writable bytes.
    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>];

    /// Assume `count` bytes are initialized.
    ///
    /// # Safety
    ///
    /// `count <= self.spare_mut().len()`.
    unsafe fn advance_mut(&mut self, count: usize);

    /// Write bytes to current buffer.
    ///
    /// # Safety
    ///
    /// `bytes.len() <= self.spare_mut().len()`.
    #[inline]
    unsafe fn write_unchecked(&mut self, bytes: &[u8]) {
        debug_assert!(bytes.len() <= self.spare_mut().len());
        unsafe {
            self.spare_mut()
                .as_mut_ptr()
                .cast::<u8>()
                .copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            self.advance_mut(bytes.len());
        }
    }

    /// Write bytes to current buffer.
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), OutOfCapacity> {
        self.spare_mut()
            .get_mut(..bytes.len())
            .ok_or(OutOfCapacity)?
            .write_copy_of_slice(bytes);
        unsafe { self.advance_mut(bytes.len()) };
        Ok(())
    }
}

impl BufMut for &mut &mut [u8] {
    #[inline]
    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe { &mut *(**self as *mut [u8] as *mut [MaybeUninit<u8>]) }
    }

    #[inline]
    unsafe fn advance_mut(&mut self, count: usize) {
        let (_, suff) = unsafe { mem::take(*self).split_at_mut_unchecked(count) };
        **self = suff;
    }
}

impl BufMut for &mut [MaybeUninit<u8>] {
    #[inline]
    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self
    }

    #[inline]
    unsafe fn advance_mut(&mut self, count: usize) {
        let (_, suff) = unsafe { mem::take(self).split_at_mut_unchecked(count) };
        *self = suff;
    }
}

impl BufMut for &mut &mut [MaybeUninit<u8>] {
    #[inline]
    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self
    }

    #[inline]
    unsafe fn advance_mut(&mut self, count: usize) {
        let (_, suff) = unsafe { mem::take(*self).split_at_mut_unchecked(count) };
        **self = suff;
    }
}
