use core::ptr::NonNull;
use core::{mem, slice};

use seryu::alloc::slab::InsertError;

use crate::alloc;

// ===== Queue =====

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

    pub fn drain(&mut self) -> impl Iterator<Item = T>
    where
        T: Copy,
    {
        let len = self.len;
        self.len = 0;
        unsafe {
            slice::from_raw_parts(self.buf.as_ptr(), len)
                .iter()
                .copied()
        }
    }
}
