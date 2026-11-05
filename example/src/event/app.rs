use core::mem;

use crate::event::EventData;
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
    fn from_data(data: u64) -> Self {
        unsafe { mem::transmute(data) }
    }

    fn into_data(self) -> u64 {
        unsafe { mem::transmute(self) }
    }
}
