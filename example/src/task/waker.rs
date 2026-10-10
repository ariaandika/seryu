use core::ptr;
use core::task::{RawWaker, RawWakerVTable, Waker};

use genos::fd::AsFd;

use crate::rt::Shared;
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

    pub fn from_waker(waker: &Waker) -> &Self {
        debug_assert!(ptr::addr_eq(waker.vtable() as *const _, &raw const VTABLE));
        unsafe { &*waker.data().cast() }
    }

    pub fn register_read<Fd: AsFd + ?Sized>(&self, fd: &Fd) {
        Shared::get().fds.register_read(fd, self.clone());
    }

    /// # Safety
    ///
    /// `self` must not be moved.
    pub unsafe fn create_waker(&self) -> Waker {
        unsafe { Waker::new(self as *const _ as *const _, &VTABLE) }
    }

    pub fn id(&self) -> TaskId {
        self.id
    }
}

static VTABLE: RawWakerVTable = {
    unsafe fn clone(data: *const ()) -> RawWaker {
        RawWaker::new(data, &VTABLE)
    }
    unsafe fn wake(_: *const ()) {
        todo!("waker `wake` virtual call")
    }
    unsafe fn wake_by_ref(_: *const ()) {
        todo!("waker `wake` virtual call")
    }
    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};
