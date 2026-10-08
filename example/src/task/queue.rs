use core::ptr::NonNull;
use core::{mem, slice};

use seryu::alloc::slab::InsertError;

use crate::alloc;

const CAP: usize = 64;

pub struct Queue<T> {
    buf: NonNull<T>,
    len: usize,
}

impl<T> Queue<T> {
    pub fn new_static() -> Self {
        const { assert!(!mem::needs_drop::<T>()) }
        Self { buf: alloc::leak_slice(CAP), len: 0 }
    }

    pub fn push(&mut self, task: T) -> Result<(), InsertError<T>> {
        if self.len == CAP {
            return Err(InsertError::new(task));
        }

        unsafe { self.buf.add(self.len).write(task) };
        self.len += 1;

        Ok(())
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.buf.as_ptr(), self.len) }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}
