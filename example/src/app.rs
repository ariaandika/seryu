use core::marker::PhantomPinned;
use core::pin::Pin;
use core::{mem, ops};

use genos::arch;
use seryu::alloc;
use seryu::event::{Epoll, Event, EventData};

use crate::alloc::Owned;
use crate::error::Error;
use crate::http::Hooks;
use crate::shared::{Global, TaskId};

// ===== EventKind =====

/// Event data.
#[derive(Clone, Copy)]
pub enum EventKind {
    Listener,
    Client(TaskId),
    Signal,
}

impl EventData for EventKind {
    fn from_raw(raw: u64) -> Self {
        unsafe { mem::transmute(raw) }
    }

    fn into_raw(self) -> u64 {
        unsafe { mem::transmute(self) }
    }
}

// ===== EventBuf =====

pub const EVENT_CAP: usize = alloc::PAGE_SIZE / 2 / size_of::<Event>();

pub type EventBuf<A> = Owned<[mem::MaybeUninit<Event>; EVENT_CAP], A>;

// ===== Rt =====

pub struct Rt {
    pub poll: Epoll,
    _p: PhantomPinned,
}

impl Rt {
    pub fn new() -> Result<Self, Error> {
        Ok(Self { poll: Epoll::new()?, _p: PhantomPinned })
    }

    pub fn setup(self: Pin<&mut Self>) {
        unsafe {
            let ptr = self.get_unchecked_mut() as *const _;
            arch::wrgsbase(ptr as u64);
        }
    }
}

// ===== GlobalRt =====

pub struct GlobalRt;

impl ops::Deref for GlobalRt {
    type Target = Rt;

    fn deref(&self) -> &Self::Target {
        unsafe { &mut *(arch::rdgsbase() as *mut _) }
    }
}

impl Global for GlobalRt {
    fn global() -> Self {
        Self
    }
}

impl Hooks for GlobalRt {
    fn added(&mut self, id: TaskId, task: &crate::http::ClientTask) {
        self.poll
            .add(&task.socket, EventKind::Client(id))
            .expect("failed to add client fd");
    }

    fn removed(&mut self, _: TaskId, task: &crate::http::ClientTask) {
        let _ = self.poll.delete(&task.socket);
    }
}
