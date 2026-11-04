use core::mem::MaybeUninit;
use core::ops;
use core::ptr::NonNull;

use seryu::alloc::Allocator;

// ===== Owned =====

pub struct Owned<T, A: Allocator> {
    ptr: NonNull<T>,
    alloc: A,
}

impl<T, A: Allocator> Drop for Owned<T, A> {
    fn drop(&mut self) {
        unsafe { self.alloc.deallocate(self.ptr) };
    }
}

impl<T, A: Allocator> Owned<T, A> {
    pub fn uninit_array<const S: usize>(
        alloc: A,
    ) -> Result<Owned<[MaybeUninit<T>; S], A>, A::Error> {
        Ok(Owned { ptr: alloc.allocate()?, alloc })
    }
}

impl<T, A: Allocator> ops::Deref for Owned<T, A> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T, A: Allocator> ops::DerefMut for Owned<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}
