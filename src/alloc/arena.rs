use core::alloc::Layout;
use core::cell::Cell;
use core::fmt;
use core::ptr::NonNull;

use crate::alloc::{Allocator, OutOfCapacity};

pub struct Arena<A: Allocator, const S: usize> {
    buf: NonNull<[u8; S]>,
    off: Cell<usize>,
    alloc: A,
}

impl<A: Allocator, const S: usize> Drop for Arena<A, S> {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.alloc.deallocate(self.buf) };
    }
}

impl<A: Allocator, const S: usize> Arena<A, S> {
    #[inline]
    pub fn new(alloc: A) -> Result<Self, A::Error> {
        Ok(Self { buf: alloc.allocate()?, off: Cell::new(0), alloc })
    }

    fn alloc(&self, layout: Layout) -> Result<NonNull<()>, OutOfCapacity> {
        let og = self.buf;
        let pad = og.align_offset(layout.align());
        let total = layout.size() + pad;

        let off = self.off.get() + total;
        if off > S {
            return Err(OutOfCapacity);
        }
        self.off.set(off);

        Ok(unsafe { og.add(pad).cast() })
    }
}

unsafe impl<A: Allocator, const S: usize> Allocator for Arena<A, S> {
    type Error = OutOfCapacity;

    #[inline]
    fn allocate<T>(&self) -> Result<NonNull<T>, Self::Error> {
        self.alloc(Layout::new::<T>()).map(|p| {
            let p = p.cast();
            debug_assert!(p.is_aligned());
            p
        })
    }

    #[inline]
    unsafe fn deallocate<T>(&self, _: NonNull<T>) {}
}

unsafe impl<A: Allocator, const S: usize> Allocator for &Arena<A, S> {
    type Error = OutOfCapacity;

    #[inline]
    fn allocate<T>(&self) -> Result<NonNull<T>, Self::Error> {
        Arena::allocate(self)
    }

    #[inline]
    unsafe fn deallocate<T>(&self, _: NonNull<T>) {}
}

impl<A: Allocator + fmt::Debug, const S: usize> fmt::Debug for Arena<A, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena").finish_non_exhaustive()
    }
}
