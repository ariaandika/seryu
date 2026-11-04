use core::alloc::Layout;
use core::ptr::NonNull;

use genos::mem;

use crate::alloc::{Allocator, OutOfMemory};

/// [`brk`] based memory allocator.
///
/// [`brk`]: genos::mem::brk
#[derive(Debug, Clone, Copy)]
pub struct Brk;

fn alloc(layout: Layout) -> Result<NonNull<()>, OutOfMemory> {
    let og = mem::brk(0 as _);
    let pad = og.align_offset(layout.align());

    let total = layout.size() + pad;
    let res = mem::brk(unsafe { og.add(total) }.as_ptr());

    if res != og { Ok(unsafe { og.add(pad).cast() }) } else { Err(OutOfMemory) }
}

unsafe impl Allocator for Brk {
    type Error = OutOfMemory;

    #[inline]
    fn allocate<T>(&self) -> Result<NonNull<T>, Self::Error> {
        alloc(Layout::new::<T>()).map(|p| {
            let p = p.cast();
            debug_assert!(p.is_aligned());
            p
        })
    }

    #[inline]
    unsafe fn deallocate<T>(&self, ptr: NonNull<T>) {
        mem::brk(ptr.as_ptr().cast());
    }
}
