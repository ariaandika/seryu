use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::{fmt, slice};

// ===== Capacity =====

/// Memory capacity in bytes.
///
/// This is implemented for `usize`. Application may implement it to constant value.
pub trait Capacity {
    fn capacity(&self) -> usize;
}

impl Capacity for usize {
    #[inline]
    fn capacity(&self) -> usize {
        *self
    }
}

// ===== Buffer =====

/// Memory initialization tracker.
pub struct Buffer<T, C> {
    ptr: NonNull<T>,
    len: usize,
    cap: C,
}

impl<T, C> Buffer<T, C> {
    /// Creates new [`Buffer`] with given raw parts.
    ///
    /// Note that the memory of this struct is unbounded. No deallocation performed on `Drop`.
    /// Caller is responsible for the memory lifetime.
    ///
    /// # Safety
    ///
    /// - `ptr` must be valid for reads and writes for `cap * size_of::<T>()` many bytes as long as
    ///   this truct lives, and must be properly aligned.
    /// - `len <= cap`, and the first `len` elements must be initialized.
    #[inline]
    pub const unsafe fn from_raw_parts(ptr: NonNull<T>, len: usize, cap: C) -> Self {
        Self { ptr, len, cap }
    }

    /// Returns the raw pointer of this buffer.
    #[inline]
    pub const fn as_ptr(&self) -> NonNull<T> {
        self.ptr
    }

    /// Returns the number of initialized elements.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Clears the buffer content, restoring original capacity.
    #[inline]
    pub const fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns the initialized elements as shared slice.
    #[inline]
    pub const fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Returns the initialized elements as mutable slice.
    #[inline]
    pub const fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    /// Extracts the buffer memory raw parts.
    #[inline]
    pub fn into_raw_parts(self) -> (NonNull<T>, usize, C) {
        (self.ptr, self.len, self.cap)
    }
}

impl<T, C: Capacity> Buffer<T, C> {
    /// Returns the total element this struct can holds.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap.capacity()
    }

    /// Returns the remaining uninitialized elements as slice.
    #[inline]
    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe {
            slice::from_raw_parts_mut(
                self.ptr.as_ptr().add(self.len).cast(),
                self.capacity() - self.len,
            )
        }
    }

    /// Set the number of initialized elements.
    ///
    /// # Safety
    ///
    /// `len <= cap`, and the first `len` elements must be initialized.
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len < self.capacity());
        self.len = new_len;
    }
}

// ===== impl trait =====

impl<T: fmt::Debug, C: Capacity> fmt::Debug for Buffer<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_list();
        f.entries(self.as_slice());
        if self.len == self.capacity() { f.finish() } else { f.finish_non_exhaustive() }
    }
}
