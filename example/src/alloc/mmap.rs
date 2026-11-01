use core::ptr::NonNull;
use core::{ffi, result};

use genos::mem::mmap;
use genos::{arch, sys};

pub type MmapError = sys::Error<arch::sys_mmap>;

pub type Result<T, E = MmapError> = result::Result<T, E>;

pub const PAGE_SIZE: usize = 4096;

// ===== core =====

pub fn mmap_anon(size: usize) -> Result<NonNull<ffi::c_void>> {
    mmap::mmap_anon(
        0 as _,
        size,
        mmap::Prot::READ | mmap::Prot::WRITE,
        mmap::Flags::PRIVATE | mmap::Flags::ANON,
    )
}

pub fn munmap(ptr: *mut ffi::c_void, size: usize) {
    let _ = mmap::munmap(ptr, size);
}

// ===== array =====

/// Allocate Memory.
///
/// The returned address are page-aligned. Its contents are initialized to zero.
pub fn array<T, const S: usize>() -> Result<NonNull<[T; S]>> {
    const { assert!(align_of::<T>() <= PAGE_SIZE) }
    mmap_anon(size_of::<[T; S]>()).map(NonNull::cast)
}

pub fn free_array<T, const S: usize>(ptr: NonNull<[T; S]>) {
    munmap(ptr.as_ptr().cast(), size_of::<[T; S]>());
}
