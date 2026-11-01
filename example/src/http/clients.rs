use core::mem::{ManuallyDrop, MaybeUninit};
use core::ptr::NonNull;
use core::{mem, ops};

use genos::net::Socket;
use seryu::alloc::OutOfCapacity;

use crate::alloc;
use crate::http::ClientTask;
use crate::task::TaskId;

pub struct Clients {
    tasks: Tasks,
    len: usize,
    last: usize,
}

impl Clients {
    pub fn new() -> alloc::Result<Self> {
        Ok(Self { tasks: Tasks::new()?, len: 0, last: 0 })
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

// ===== Chunks =====

const TASKS_CAP: usize = 16;

struct Tasks(NonNull<[Task; TASKS_CAP]>);

impl Tasks {
    fn new() -> alloc::Result<Self> {
        alloc::array().map(Self)
    }
}

impl ops::Deref for Tasks {
    type Target = [Task; TASKS_CAP];

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl ops::DerefMut for Tasks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
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
