use genos::fd::{AsFd, BorrowedFd, RawFd};
use seryu::alloc::Slab;

use crate::alloc;
use crate::rt::task::TaskId;

// ===== FdProp =====

#[derive(Clone)]
pub struct FdProp {
    pub fd: RawFd,
    pub read_task: Option<TaskId>,
    pub write_task: Option<TaskId>,
}

impl FdProp {
    pub fn new(fd: RawFd) -> Self {
        Self { fd, read_task: None, write_task: None }
    }

    pub fn new_read(fd: RawFd, task: TaskId) -> Self {
        Self { fd, read_task: Some(task), write_task: None }
    }
}

impl AsFd for FdProp {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}

// ===== FdTable =====

const SIZE: usize = 64;
const FD_OFF: i32 = 3/*std stream*/;

pub struct FdTable {
    buf: Slab<FdProp>,
}

impl FdTable {
    pub fn new_static() -> Self {
        let ptr = alloc::leak_slice::<FdProp>(SIZE).as_ptr().cast();
        Self { buf: unsafe { Slab::from_buf(ptr, size_of::<FdTable>() * SIZE) } }
    }

    pub fn get_mut(&mut self, fd: RawFd) -> Option<&mut FdProp> {
        self.buf.get_mut(fd as usize)
    }

    pub fn insert<Fd: AsFd>(&mut self, fd: &Fd) -> RawFd {
        let fd = fd.as_raw_fd();
        let key = fd.strict_sub(FD_OFF) as usize;
        assert_eq!(key, self.buf.peek_key());
        self.buf.insert(FdProp::new(fd)).unwrap() as RawFd
    }

    pub fn insert_read<Fd: AsFd>(&mut self, fd: &Fd, task: TaskId) -> RawFd {
        let fd = fd.as_raw_fd();
        let key = fd.strict_sub(FD_OFF) as usize;
        // BUG: slab uses the "last deleted" key, while fd uses "lowest value" key
        assert_eq!(key, self.buf.peek_key());
        self.buf.insert(FdProp::new_read(fd, task)).unwrap() as RawFd
    }

    pub fn remove(&mut self, fd: RawFd) -> Option<FdProp> {
        let key = fd.strict_sub(FD_OFF) as usize;
        self.buf.remove(key)
    }
}
