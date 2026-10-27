use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::slice;
use core::task::{Context, Poll};

use genos::fd::AsFd;
use genos::io;
use seryu::alloc::OutOfCapacity;
use seryu::bytes::BufMut;
use seryu::error::WriteError;
use seryu::postgres::Encode;

use crate::task::ContextExt;

// ===== WriteBuf =====

pub struct WriteBuf {
    ptr: NonNull<u8>,
    len: usize,
    off: usize,
}

// ===== getters/setters =====

impl WriteBuf {
    pub const CAPACITY: usize = 1024;

    pub unsafe fn from_raw(ptr: NonNull<u8>) -> Self {
        Self { ptr, len: 0, off: 0 }
    }

    pub fn clear(&mut self) {
        self.ptr = unsafe { self.ptr.sub(self.off) };
        self.len = 0;
        self.off = 0;
    }
}

// ===== impl BufMut =====

impl BufMut for &mut WriteBuf {
    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe {
            let ptr = self.ptr.as_ptr().add(self.len);
            let spare = WriteBuf::CAPACITY - self.off - self.len;
            slice::from_raw_parts_mut(ptr.cast(), spare)
        }
    }

    unsafe fn advance_mut(&mut self, count: usize) {
        self.len += count;
    }
}

// ===== OS =====

impl WriteBuf {
    /// Flush buffer.
    pub fn poll_flush<Fd: AsFd>(
        &mut self,
        fd: &Fd,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), WriteError>> {
        match self.flush_sync(fd) {
            Ok(ok) => Poll::Ready(Ok(ok)),
            Err(err) => {
                if err.code().is_retry() {
                    cx.register_task_write();
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err))
                }
            }
        }
    }

    pub fn flush_sync<Fd: AsFd>(&mut self, fd: &Fd) -> Result<(), WriteError> {
        while self.len != 0 {
            let bytes = unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) };
            let write = io::write(fd, bytes)?;
            self.ptr = unsafe { self.ptr.add(write) };
            self.len -= write;
            self.off += write;
        }
        self.clear();
        Ok(())
    }
}

// ===== protocol =====

impl WriteBuf {
    pub fn write_msg<M: Encode>(&mut self, msg: M) -> Result<(), OutOfCapacity> {
        msg.encode_msg(self)
    }
}
