use core::task::{RawWaker, RawWakerVTable, Waker};

// ===== Global =====

pub trait Global {
    fn global() -> Self;
}

// ===== TaskId =====

#[derive(Debug, Clone, Copy)]
pub struct TaskId(u32);

impl TaskId {
    pub fn from_idx(idx: usize) -> Self {
        Self(idx.try_into().unwrap())
    }

    pub fn to_idx(self) -> usize {
        self.0 as usize
    }
}

// ===== Waker =====

pub fn waker<T>(task: *const T) -> Waker {
    unsafe { Waker::new(task.cast(), &VTABLE) }
}

static VTABLE: RawWakerVTable = {
    unsafe fn clone(data: *const ()) -> RawWaker {
        RawWaker::new(data, &VTABLE)
    }
    unsafe fn wake(_: *const ()) {
        unimplemented!("waker `wake` virtual call")
    }
    unsafe fn wake_by_ref(_: *const ()) {
        unimplemented!("waker `wake` virtual call")
    }
    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};
