use core::mem::{self, ManuallyDrop, MaybeUninit};

use seryu::alloc::OutOfCapacity;
use seryu::os::Socket;

use crate::alloc::{self, Mmap};
use crate::http::ClientTask;
use crate::task::TaskId;

const TASKS_CAP: usize = 16;

pub struct Clients {
    tasks: Mmap<[Task; TASKS_CAP]>,
    len: usize,
    last: usize,
}

impl Clients {
    pub fn new() -> alloc::Result<Self> {
        // SAFETY: `Task` is valid with zeroed byte-pattern
        let tasks = unsafe { Mmap::zeroed()? };
        Ok(Self { tasks, len: 0, last: 0 })
    }

    pub fn peek_id(&self) -> TaskId {
        TaskId::from_idx(self.last)
    }

    pub fn create(&mut self, socket: Socket) -> Result<usize, OutOfCapacity> {
        let Some(task) = self.tasks.get_mut(self.last) else {
            return Err(OutOfCapacity);
        };

        assert!(task.is_none());

        let next = if self.last == self.len {
            // appending
            self.len += 1;
            self.len
        } else {
            // recycling slot
            task.metadata()
        };

        ClientTask::initialize(socket, task.as_task_uninit());
        assert!(!task.is_none());

        Ok(mem::replace(&mut self.last, next))
    }

    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut ClientTask> {
        let chunk = self.tasks.get_mut(id.to_idx())?;
        assert!(!chunk.is_none());
        Some(unsafe { chunk.as_task() })
    }

    pub fn delete(&mut self, idx: usize) {
        let Some(chunk) = self.tasks.get_mut(idx) else {
            return;
        };
        if chunk.is_none() {
            return;
        }
        unsafe { chunk.task.assume_init_drop() };
        chunk.set_none(self.last);
        self.last = idx;
    }
}

// ===== Task =====

union Task {
    task: ManuallyDrop<MaybeUninit<ClientTask>>,
    tag: (usize, usize),
}

impl Task {
    unsafe fn as_task(&mut self) -> &mut ClientTask {
        unsafe { self.task.assume_init_mut() }
    }

    fn as_task_uninit(&mut self) -> &mut MaybeUninit<ClientTask> {
        unsafe { &mut self.task }
    }

    fn is_none(&self) -> bool {
        unsafe { self.tag.0 == 0 }
    }

    fn metadata(&self) -> usize {
        unsafe { self.tag.1 }
    }

    fn set_none(&mut self, metadata: usize) {
        self.tag = (0, metadata);
    }
}
