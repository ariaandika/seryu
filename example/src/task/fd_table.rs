use core::ptr::NonNull;

use genos::fd::{AsFd, BorrowedFd, RawFd};
use seryu::alloc::slab::InsertError;

use crate::alloc;
use crate::task::WakerData;

// ===== FdProp =====

#[derive(Clone)]
pub struct FdProp {
    pub fd: RawFd,
    pub read: Option<WakerData>,
    pub write: Option<WakerData>,
}

impl FdProp {
    fn new(fd: RawFd) -> Self {
        Self { fd, read: None, write: None }
    }
}

impl AsFd for FdProp {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}

// ===== FdTable =====

const CAP: usize = 64;
const FD_OFF: i32 = 3/*std stream*/;

pub struct FdTable {
    ptr: NonNull<FdProp>,
}

impl FdTable {
    pub fn new_static() -> Self {
        Self { ptr: alloc::leak_slice::<FdProp>(CAP) }
    }

    pub fn get_mut(&mut self, fd: RawFd) -> &mut FdProp {
        let idx = fd.strict_sub(FD_OFF) as usize;
        if idx >= CAP {
            panic!("`FdTable::get_mut` out of bounds");
        }
        unsafe { self.ptr.add(idx).as_mut() }
    }

    pub fn insert<Fd: AsFd>(&mut self, fd: &Fd) -> Result<RawFd, InsertError<()>> {
        let fd = fd.as_raw_fd();
        let idx = fd.strict_sub(FD_OFF) as usize;
        if idx >= CAP {
            return Err(InsertError::new(()));
        }
        unsafe { self.ptr.add(idx).write(FdProp::new(fd)) };
        Ok(fd)
    }

    pub fn register_read<Fd: AsFd + ?Sized>(&mut self, fd: &Fd, waker: WakerData) {
        // TODO: multiple waker for each fd
        self.get_mut(fd.as_raw_fd()).read = Some(waker);
    }

    pub fn remove(&mut self, _fd: RawFd) {
        // currently noop
    }
}
