use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::slice;

use seryu::alloc::OutOfMemory;

// ===== Memory =====

const ARENA_CAP: usize = 4096;

pub struct Arena {
    ptr: NonNull<u8>,
    len: usize,
}

impl Arena {
    pub const CAPACITY: usize = ARENA_CAP;

    pub(super) fn new(ptr: NonNull<u8>) -> Self {
        Self { ptr, len: 0 }
    }

    pub fn alloc(&mut self, len: usize) -> Result<NonNull<u8>, OutOfMemory> {
        let ptr = unsafe { self.ptr.add(self.len) };
        self.len += align8(len);
        if self.len <= ARENA_CAP {
            Ok(ptr)
        } else {
            self.len -= len;
            Err(OutOfMemory)
        }
    }

    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe {
            let ptr = self.ptr.add(self.len).as_ptr().cast();
            slice::from_raw_parts_mut(ptr, ARENA_CAP - self.len)
        }
    }

    pub fn as_ptr(&self) -> NonNull<u8> {
        self.ptr
    }
}

const fn align8(value: usize) -> usize {
    (value + 7) & !7
}
