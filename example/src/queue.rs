use core::mem::MaybeUninit;
use core::{mem, slice};

use seryu::alloc::slab::InsertError;

use crate::alloc::{self, Mmap};

// ===== Queue =====

const SIZE: usize = alloc::PAGE_SIZE;
const CAP: usize = SIZE / 8;

pub struct Queue<T> {
    buf: Mmap<[MaybeUninit<T>; CAP]>,
    len: usize,
}

impl<T> Queue<T> {
    pub fn new() -> alloc::Result<Self> {
        const { assert!(!mem::needs_drop::<T>()) }
        Mmap::array().map(|buf| Self { buf, len: 0 })
    }

    pub fn push(&mut self, task: T) -> Result<(), InsertError<T>> {
        let Some(task_mut) = self.buf.get_mut(self.len) else {
            return Err(InsertError::new(task));
        };
        task_mut.write(task);
        self.len += 1;
        Ok(())
    }

    pub fn drain(&mut self) -> impl Iterator<Item = T>
    where
        T: Copy,
    {
        let len = self.len;
        self.len = 0;
        unsafe { slice::from_raw_parts(self.buf.as_ptr().cast(), len) }
            .iter()
            .copied()
    }
}
