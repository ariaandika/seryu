use core::ptr::NonNull;
use core::{mem, slice};

use seryu::alloc::slab::InsertError;

use crate::alloc;

// ===== Queue =====

const SIZE: usize = alloc::PAGE_SIZE;
const CAP: usize = SIZE / 8;

pub struct Queue<T> {
    buf: NonNull<[T; CAP]>,
    len: usize,
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        alloc::free_array(self.buf);
    }
}

impl<T> Queue<T> {
    pub fn new() -> alloc::Result<Self> {
        const { assert!(!mem::needs_drop::<T>()) }
        alloc::array().map(|buf| Self { buf, len: 0 })
    }

    fn as_ptr(&mut self) -> *mut T {
        self.buf.as_ptr().cast()
    }

    pub fn push(&mut self, task: T) -> Result<(), InsertError<T>> {
        if self.len == CAP {
            return Err(InsertError::new(task));
        }

        unsafe { self.as_ptr().add(self.len).write(task) };
        self.len += 1;

        Ok(())
    }

    pub fn drain(&mut self) -> impl Iterator<Item = T>
    where
        T: Copy,
    {
        let len = self.len;
        self.len = 0;
        unsafe { slice::from_raw_parts(self.as_ptr(), len) }
            .iter()
            .copied()
    }
}
