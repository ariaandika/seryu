use core::task::{Context, Waker};

use genos::fd::AsFd;

use crate::task::{FdTable, WakerData};

// ===== ContextExt =====

pub trait ContextExt {
    fn data(&mut self) -> &mut ContextData<'_>;

    fn register_read<Fd: AsFd + ?Sized>(&mut self, fd: &Fd) {
        let data = self.data();
        data.fds
            .register_read(fd, WakerData::from_waker(data.waker).clone());
    }

    fn register_write<Fd: AsFd + ?Sized>(&mut self, fd: &Fd) {
        let data = self.data();
        data.fds
            .register_write(fd, WakerData::from_waker(data.waker).clone());
    }
}

impl ContextExt for Context<'_> {
    fn data(&mut self) -> &mut ContextData<'_> {
        assert!(WakerData::is(self.waker()));
        unsafe { &mut *(self as *mut _ as *mut _) }
    }
}

// ===== ContextData =====

pub struct ContextData<'a> {
    waker: &'a Waker,
    fds: &'a mut FdTable,
}

impl<'a> ContextData<'a> {
    pub fn new(waker: &'a Waker, fds: &'a mut FdTable) -> Self {
        Self { waker, fds }
    }

    pub fn as_mut_context(&mut self) -> &mut Context<'a> {
        unsafe { &mut *(self as *mut _ as *mut _) }
    }
}
