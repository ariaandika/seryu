use seryu::alloc::Slab;

use crate::alloc;
use crate::http::Task;

// ===== TaskId =====

#[derive(Debug, Clone, Copy)]
pub struct TaskId(u32);

impl TaskId {
    fn new(id: u32) -> Self {
        Self(id)
    }

    fn raw(self) -> u32 {
        self.0
    }
}

// ===== Tasks =====

const CAP: usize = 512;

pub struct Tasks {
    buf: Slab<Task>,
}

impl Tasks {
    pub fn new_static() -> Self {
        let ptr = alloc::leak_slice::<Task>(512).as_ptr().cast();
        let buf = unsafe { Slab::from_buf(ptr, CAP * size_of::<Task>()) };
        Self { buf }
    }

    pub fn peek_id(&self) -> TaskId {
        TaskId::new(self.buf.peek_key().try_into().unwrap())
    }

    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.buf.get_mut(id.raw() as usize)
    }

    pub fn insert(&mut self, task: Task) -> TaskId {
        self.buf
            .insert(task)
            .map(|id| TaskId::new(id.try_into().unwrap()))
            .unwrap()
    }

    pub fn remove(&mut self, id: TaskId) -> Option<Task> {
        self.buf.remove(id.raw() as usize)
    }
}
