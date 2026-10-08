use seryu::alloc::Slab;

use crate::alloc;

// ===== TaskId =====

#[derive(Debug, Clone, Copy)]
pub struct TaskId(u32);

impl TaskId {
    pub(super) fn new(id: u32) -> Self {
        Self(id)
    }

    pub(super) fn raw(self) -> u32 {
        self.0
    }
}

// ===== Tasks =====

const CAP: usize = 512;

pub struct Tasks<T> {
    buf: Slab<T>,
}

impl<T> Tasks<T> {
    pub fn new_static() -> Self {
        let ptr = alloc::leak_slice::<T>(512).as_ptr().cast();
        let buf = unsafe { Slab::from_buf(ptr, CAP * size_of::<T>()) };
        Self { buf }
    }

    pub fn peek_id(&self) -> TaskId {
        TaskId::new(self.buf.peek_key().try_into().unwrap())
    }

    pub fn insert(&mut self, task: T) -> TaskId {
        self.buf
            .insert(task)
            .map(|id| TaskId::new(id.try_into().unwrap()))
            .unwrap()
    }

    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut T> {
        self.buf.get_mut(id.raw() as usize)
    }

    pub fn remove(&mut self, id: TaskId) -> Option<T> {
        self.buf.remove(id.raw() as usize)
    }
}
