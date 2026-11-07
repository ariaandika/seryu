use core::mem::{self, ManuallyDrop, MaybeUninit};
use core::pin::Pin;

use seryu::alloc::OutOfCapacity;
use seryu::event::{Event, Interest};
use seryu::os::Socket;

use crate::alloc::{self, Mmap};
use crate::error::Error;
use crate::event::EventKind;
use crate::http::ClientTask;
use crate::rt::Reactor;
use crate::task::TaskId;

const TASKS_CAP: usize = 16;

pub struct Clients<R> {
    tasks: Mmap<[Task; TASKS_CAP]>,
    len: usize,
    last: usize,
    reactor: R,
}

impl<R: Reactor> Clients<R> {
    pub fn new(reactor: R) -> alloc::Result<Self> {
        // SAFETY: `Task` is valid with zeroed byte-pattern
        let tasks = unsafe { Mmap::zeroed()? };
        Ok(Self { tasks, len: 0, last: 0, reactor })
    }

    pub fn create(&mut self, socket: Socket) -> Result<usize, Error> {
        let Some(task) = self.tasks.get_mut(self.last) else {
            return Err(OutOfCapacity.into());
        };

        // reactor synchronization
        let data = EventKind::Client(TaskId::from_idx(self.last));
        self.reactor.add(&socket, data)?;

        assert!(task.is_none());

        let next = if self.last == self.len {
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
        ClientTask::initialize(socket, slot);

        assert!(task.is_some());

        Ok(mem::replace(&mut self.last, next))
    }

    pub fn delete(&mut self, id: TaskId) {
        let idx = id.to_idx();
        let Some(slot) = self.tasks.get_mut(idx) else {
            return;
        };
        if slot.is_none() {
            return;
        }

        // SAFETY: `slot.is_some()`
        let task = unsafe { slot.task_mut() };

        // reactor synchronization
        let _ = self.reactor.delete(&task.socket);

        // SAFETY: `slot.is_some()`
        unsafe { slot.drop_in_place(self.last) };

        self.last = idx;
    }

    pub fn get_mut(&mut self, id: TaskId) -> Option<Pin<&mut ClientTask>> {
        self.tasks
            .get_mut(id.to_idx())
            .map(|t| Pin::new(t.assert_task_mut()))
    }

    /// Returns `true` if given task should be scheduled.
    pub fn schedule_client(&mut self, id: TaskId, event: &Event) -> bool {
        let Some(task) = self.get_mut(id) else {
            return false;
        };
        let task = task.get_mut();
        let interest = task.interest & event.interest();

        let want_rd = interest.has(Interest::READ);
        let want_wr = interest.has(Interest::WRITE);

        // prevent not interested event
        if !(want_rd | want_wr) {
            return false;
        }

        let unset = (Interest::READ ^ interest) | (Interest::WRITE ^ interest);
        task.interest = Interest::default() | unset;
        true
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
