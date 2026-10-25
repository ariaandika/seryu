use core::ptr;
use core::task::{RawWaker, RawWakerVTable, Waker};

use crate::task::TaskId;

pub struct WakerData {
    id: TaskId,
}

impl Clone for WakerData {
    fn clone(&self) -> Self {
        Self { id: self.id }
    }
}

impl WakerData {
    pub fn new(id: TaskId) -> Self {
        Self { id }
    }

    pub fn is(waker: &Waker) -> bool {
        ptr::addr_eq(waker.vtable() as *const _, &raw const VTABLE)
    }

    /// # Safety
    ///
    /// `self` must not be moved for the lifetime of the returned waker.
    pub unsafe fn create_waker(&self) -> Waker {
        unsafe { Waker::new(self as *const _ as *const _, &VTABLE) }
    }
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
