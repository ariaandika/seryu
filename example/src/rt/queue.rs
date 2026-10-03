use core::ptr::NonNull;
use std::slice;

use seryu::alloc::slab::InsertError;

use crate::alloc;
use crate::rt::task::TaskId;

const CAP: usize = 64;

pub struct Queue {
    buf: NonNull<TaskId>,
    len: usize,
}

impl Queue {
    pub fn new_static() -> Self {
        Self { buf: alloc::leak_slice(CAP), len: 0 }
    }

    pub fn push(&mut self, task: TaskId) -> Result<(), InsertError<TaskId>> {
        if self.len == CAP {
            return Err(InsertError::new(task));
        }

        unsafe { self.buf.add(self.len).write(task) };
        self.len += 1;

        Ok(())
    }

    pub fn as_slice(&self) -> &[TaskId] {
        unsafe { slice::from_raw_parts(self.buf.as_ptr(), self.len) }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}
