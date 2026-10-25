use seryu::alloc::Slab;

use crate::alloc;
use crate::task::TaskId;

// ===== Tasks =====

const CAP: usize = 512;

pub struct Tasks<T> {
    buf: Slab<T>,
}

impl<T> Drop for Tasks<T> {
    fn drop(&mut self) {
        alloc::free_array::<T, CAP>(self.buf.as_non_null().cast());
    }
}

impl<T> Tasks<T> {
    pub fn new() -> alloc::Result<Self> {
        let ptr = alloc::array::<T, CAP>()?.as_ptr().cast();
        Ok(Self { buf: unsafe { Slab::from_buf(ptr, size_of::<[T; CAP]>()) } })
    }

    pub fn peek_id(&self) -> TaskId {
        TaskId::from_idx(self.buf.peek_key())
    }

    pub fn insert(&mut self, task: T) -> TaskId {
        self.buf.insert(task).map(TaskId::from_idx).unwrap()
    }

    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut T> {
        self.buf.get_mut(id.to_idx())
    }

    pub fn remove(&mut self, id: TaskId) -> Option<T> {
        self.buf.remove(id.to_idx())
    }
}
