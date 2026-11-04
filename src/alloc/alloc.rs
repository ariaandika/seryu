use core::error;
use core::ptr::NonNull;

/// Dynamic Memory Allocator.
///
/// # Safety
///
/// See [`core::alloc::Allocator`].
pub unsafe trait Allocator {
    type Error: error::Error;

    /// Attempts to allocate a block of memory.
    fn allocate<T>(&self) -> Result<NonNull<T>, Self::Error>;

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// See [`core::alloc::Allocator::deallocate`].
    unsafe fn deallocate<T>(&self, ptr: NonNull<T>);
}
