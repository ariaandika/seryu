use core::mem::{ManuallyDrop, MaybeUninit};
use core::pin::Pin;
use core::task::Context;

use seryu::alloc::OutOfCapacity;
use seryu::event::{Event, Interest};
use seryu::os::Socket;

use crate::alloc::{self, Mmap};
use crate::error::Error;
use crate::http::{ClientTask, Spawner};
use crate::shared::{self, Global, TaskId};

const TASKS_CAP: usize = 16;

// ===== Hooks =====

pub trait Hooks {
    fn added(&mut self, id: TaskId, task: &ClientTask);

    fn removed(&mut self, id: TaskId, task: &ClientTask);
}

// ===== Clients =====

pub struct Clients<H> {
    tasks: Mmap<[Task; TASKS_CAP]>,
    len: usize,
    last: usize,
    hooks: H,
}

impl<H: Hooks + Global> Clients<H> {
    pub fn new() -> alloc::Result<Self> {
        Self::from_parts(H::global())
    }
}

impl<H: Hooks> Clients<H> {
    pub fn from_parts(hooks: H) -> alloc::Result<Self> {
        // SAFETY: `Task` is valid with zeroed byte-pattern
        let tasks = unsafe { Mmap::zeroed()? };
        Ok(Self { tasks, len: 0, last: 0, hooks })
    }

    pub fn create(&mut self, socket: Socket) -> Result<(), Error> {
        let Some(task) = self.tasks.get_mut(self.last) else {
            return Err(OutOfCapacity.into());
        };
        let new_id = TaskId::from_idx(self.last);

        assert!(task.is_none());

        self.last = if self.last == self.len {
            // appending
            self.len += 1;
            self.len
        } else {
            // recycling slot
            // SAFETY: `task.is_none()`
            unsafe { task.metadata() }
        };

        // SAFETY: `task.is_none()`
        let slot = unsafe { task.as_task_uninit() };
        let client = ClientTask::initialize(socket, slot);
        self.hooks.added(new_id, client);

        debug_assert!(task.is_some());

        Ok(())
    }

    pub fn delete(&mut self, id: TaskId) {
        let idx = id.to_idx();
        let Some(slot) = self.tasks.get_mut(idx) else {
            return;
        };

        assert!(slot.is_some());

        // SAFETY: `slot.is_some()`
        let task = unsafe { slot.task_mut() };
        self.hooks.removed(id, task);

        // SAFETY: `slot.is_some()`
        unsafe { slot.drop_in_place(self.last) };

        debug_assert!(slot.is_none());

        self.last = idx;
    }

    pub fn get_mut(&mut self, id: TaskId) -> Option<Pin<&mut ClientTask>> {
        self.tasks
            .get_mut(id.to_idx())
            .map(|t| Pin::new(t.assert_task_mut()))
    }
}

impl<H: Hooks> Spawner for &mut Clients<H> {
    fn spawn(&mut self, socket: Socket) {
        Clients::create(*self, socket).expect("failed to create client");
    }
}

// ===== Future =====

impl<H: Hooks> Clients<H> {
    pub fn poll(&mut self, id: TaskId, event: &Event) {
        let Some(mut pin_task) = self.get_mut(id) else {
            return;
        };
        let task = pin_task.as_mut().get_mut();
        let interest = task.interest & event.interest();

        // prevent not interested event
        if !(interest.has(Interest::READ) | interest.has(Interest::WRITE)) {
            return;
        }

        // unset the emitted event
        task.interest = (task.interest & event.interest()) ^ event.interest();

        let waker = shared::waker(task);
        let mut cx = Context::from_waker(&waker);

        if pin_task.poll(&mut cx).is_ready() {
            self.delete(id)
        }
    }
}

// ===== Task =====

union Task {
    task: ManuallyDrop<MaybeUninit<ClientTask>>,
    tag: (usize, usize),
}

impl Task {
    fn is_none(&self) -> bool {
        unsafe { self.tag.0 == 0 }
    }

    fn is_some(&self) -> bool {
        unsafe { self.tag.0 != 0 }
    }

    // ===== tag =====

    /// `self.is_none()`
    unsafe fn metadata(&self) -> usize {
        unsafe { self.tag.1 }
    }

    /// `self.is_none()`
    unsafe fn as_task_uninit(&mut self) -> &mut MaybeUninit<ClientTask> {
        unsafe { &mut self.task }
    }

    // ===== task =====

    /// `self.is_some()`
    fn assert_task_mut(&mut self) -> &mut ClientTask {
        assert!(self.is_some());
        unsafe { self.task_mut() }
    }

    /// `self.is_some()`
    unsafe fn task_mut(&mut self) -> &mut ClientTask {
        unsafe { self.task.assume_init_mut() }
    }

    /// `self.is_some()`
    unsafe fn drop_in_place(&mut self, metadata: usize) {
        debug_assert!(self.is_some());
        unsafe { self.task.assume_init_drop() };
        self.tag = (0, metadata);
    }
}
