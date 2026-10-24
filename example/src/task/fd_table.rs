use core::ptr::NonNull;

use genos::fd::{AsFd, BorrowedFd, RawFd};

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

const CAP: usize = alloc::PAGE_SIZE / size_of::<FdProp>();
const FD_OFF: i32 = 3/*std stream*/;

pub struct FdTable {
    buf: NonNull<[FdProp; CAP]>,
}

impl Drop for FdTable {
    fn drop(&mut self) {
        alloc::free_array(self.buf);
    }
}

impl FdTable {
    pub fn new() -> alloc::Result<Self> {
        alloc::array().map(|buf| Self { buf })
    }

    fn as_mut_ptr(&mut self) -> NonNull<FdProp> {
        self.buf.cast()
    }

    pub fn get_mut(&mut self, fd: RawFd) -> &mut FdProp {
        let idx = fd.strict_sub(FD_OFF) as usize;
        assert!(idx < CAP, "FdProp out of bounds");
        unsafe { self.as_mut_ptr().add(idx).as_mut() }
    }

    pub fn register_read<Fd: AsFd + ?Sized>(&mut self, fd: &Fd, waker: WakerData) {
        // TODO: multiple waker for each fd
        self.get_mut(fd.as_raw_fd()).read = Some(waker);
    }

    pub fn register_write<Fd: AsFd + ?Sized>(&mut self, fd: &Fd, waker: WakerData) {
        self.get_mut(fd.as_raw_fd()).write = Some(waker);
    }
}

impl FdTable {
    pub fn insert<'a, Fd: AsFd>(&'a mut self, fd: &Fd) -> &'a mut FdProp {
        let fd = fd.as_raw_fd();
        let idx = fd.strict_sub(FD_OFF) as usize;
        assert!(idx < CAP, "FdProp out of bounds");
        unsafe {
            let mut ptr = self.as_mut_ptr().add(idx);
            ptr.write(FdProp::new(fd));
            ptr.as_mut()
        }
    }

    pub fn remove(&mut self, _fd: RawFd) {
        // currently noop
    }
}
