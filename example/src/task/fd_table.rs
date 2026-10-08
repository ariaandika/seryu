use core::ptr::NonNull;

use genos::fd::{AsFd, BorrowedFd, RawFd};
use seryu::alloc::slab::InsertError;

use crate::alloc;
use crate::task::TaskId;

// ===== FdProp =====

#[derive(Clone)]
pub struct FdProp {
    pub fd: RawFd,
    pub read_task: Option<TaskId>,
    pub write_task: Option<TaskId>,
}

impl FdProp {
    fn new(fd: RawFd, read_task: Option<TaskId>) -> Self {
        Self { fd, read_task, write_task: None }
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
        unsafe { self.ptr.add(idx).as_mut() }
    }

    pub fn insert<Fd: AsFd>(&mut self, fd: &Fd) -> Result<RawFd, InsertError<()>> {
        self.insert_inner(fd, None)
    }

    pub fn insert_read<Fd: AsFd>(&mut self, fd: &Fd, task: TaskId) -> Result<i32, InsertError<()>> {
        self.insert_inner(fd, Some(task))
    }

    fn insert_inner<Fd: AsFd>(
        &mut self,
        fd: &Fd,
        task: Option<TaskId>,
    ) -> Result<RawFd, InsertError<()>> {
        let fd = fd.as_raw_fd();
        let idx = fd.strict_sub(FD_OFF) as usize;
        if idx >= CAP {
            return Err(InsertError::new(()));
        }
        unsafe { self.ptr.add(idx).write(FdProp::new(fd, task)) };
        Ok(fd)
    }

    pub fn remove(&mut self, _fd: RawFd) {
        // currently noop
    }
}
