use core::mem::MaybeUninit;
use core::slice;

use seryu::alloc::Allocator;
use seryu::alloc::slab::InsertError;

use crate::alloc::{self, Owned};

// ===== Queue =====

const SIZE: usize = alloc::PAGE_SIZE / 4;
const CAP: usize = SIZE / 8;

pub struct Queue<T, A: Allocator> {
    buf: Owned<[MaybeUninit<T>; CAP], A>,
    len: usize,
}

impl<T, A: Allocator> Queue<T, A> {
    pub fn new(alloc: A) -> Result<Self, A::Error> {
        Ok(Self { buf: Owned::uninit_array(alloc)?, len: 0 })
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
