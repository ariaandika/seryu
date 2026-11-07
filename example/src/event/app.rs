use core::mem;

use seryu::alloc;
use seryu::event::{Event, EventData};

use crate::alloc::Owned;
use crate::task::TaskId;

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
