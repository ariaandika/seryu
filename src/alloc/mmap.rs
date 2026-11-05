use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::{fmt, ops, result};

use genos::mem::mmap;
use genos::{arch, sys};

use crate::alloc::Allocator;

pub type MmapError = sys::Error<arch::sys_mmap>;

pub type Result<T, E = MmapError> = result::Result<T, E>;

pub const PAGE_SIZE: usize = 4096;

// ===== core =====

fn mmap_anon<T>() -> Result<NonNull<T>> {
    const { assert!(align_of::<T>() <= PAGE_SIZE) }
    mmap::mmap_anon(
        0 as _,
        size_of::<T>(),
        mmap::Prot::READ | mmap::Prot::WRITE,
        mmap::Flags::PRIVATE | mmap::Flags::ANON,
    )
    .map(NonNull::cast)
}

fn munmap<T>(ptr: *mut T) {
    let _ = mmap::munmap(ptr.cast(), size_of::<T>());
}

// ===== Mmap =====

#[repr(transparent)]
pub struct Mmap<T>(NonNull<T>);

impl<T> Drop for Mmap<T> {
    #[inline]
    fn drop(&mut self) {
        munmap(self.0.as_ptr());
    }
}

impl<T> Mmap<T> {
    /// Allocate new `Mmap` with its bytes set to zero.
    ///
    /// # Safety
    ///
    /// `T` must be valid in all-zero byte-pattern.
    #[inline]
    pub unsafe fn zeroed() -> Result<Self> {
        mmap_anon().map(Self)
    }
}

impl<T, const N: usize> Mmap<[MaybeUninit<T>; N]> {
    #[inline]
    pub fn array() -> Result<Self> {
        mmap_anon().map(Self)
    }
}

impl<T> ops::Deref for Mmap<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<T> ops::DerefMut for Mmap<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

impl<T: fmt::Debug> fmt::Debug for Mmap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

// ===== Allocator =====

pub struct MmapAlloc;

unsafe impl Allocator for MmapAlloc {
    type Error = MmapError;

    #[inline]
    fn allocate<T>(&self) -> Result<NonNull<T>, Self::Error> {
        mmap_anon()
    }

    #[inline]
    unsafe fn deallocate<T>(&self, ptr: NonNull<T>) {
        munmap(ptr.as_ptr());
    }
}
