use genos::fd::{AsFd, BorrowedFd, RawFd};
use seryu::alloc::Slab;

use crate::alloc;

// ===== FdProp =====

#[repr(align(8))]
pub struct FdProp {
    raw: RawFd,
}

impl AsFd for FdProp {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.raw) }
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

    pub fn insert<Fd: AsFd>(&mut self, fd: &Fd) {
        let raw = fd.as_raw_fd();
        let key = raw.strict_sub(FD_OFF) as usize;
        assert_eq!(key, self.buf.peek_key());
        self.buf.insert(FdProp { raw }).unwrap();
    }

    pub fn remove<Fd: AsFd>(&mut self, fd: &Fd) {
        let raw = fd.as_raw_fd();
        let key = raw.strict_sub(FD_OFF) as usize;
        assert!(self.buf.remove(key).is_some());
    }
}
