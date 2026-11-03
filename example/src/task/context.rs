use core::task::{Context, Waker};

use crate::task::{Interest, WakerData};

// ===== ContextExt =====

pub trait ContextExt {
    fn data_mut(&mut self) -> &mut ContextData<'_>;

    fn register_task_read(&mut self) {
        self.data_mut().state |= Interest::READ;
    }

    fn register_task_write(&mut self) {
        self.data_mut().state |= Interest::WRITE;
    }
}

impl ContextExt for Context<'_> {
    fn data_mut(&mut self) -> &mut ContextData<'_> {
        assert!(WakerData::is(self.waker()));
        unsafe { &mut *(self as *mut _ as *mut _) }
    }
}

// ===== ContextData =====

#[repr(C)]
pub struct ContextData<'a> {
    waker: &'a Waker,
    state: Interest,
}

impl<'a> ContextData<'a> {
    pub fn new(waker: &'a Waker) -> Self {
        Self { waker, state: Interest::EMPTY }
    }

    pub fn as_mut_context(&mut self) -> &mut Context<'a> {
        unsafe { &mut *(self as *mut _ as *mut _) }
    }
}
