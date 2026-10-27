use core::ptr::NonNull;
use core::slice;
use core::task::{Context, Poll};

use genos::fd::AsFd;
use genos::io;
use seryu::bytes::Buf;
use seryu::error::ReadError;
use seryu::postgres::DecodeError;
use seryu::postgres::backend::Message;

use crate::task::ContextExt;

// ===== ReadBuf =====

pub struct ReadBuf {
    ptr: NonNull<u8>,
    len: usize,
    off: usize,
}

// ===== getters/setters =====

impl ReadBuf {
    pub const CAPACITY: usize = 1024;

    pub unsafe fn from_raw(ptr: NonNull<u8>) -> Self {
        Self { ptr, len: 0, off: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

// ===== impl Buf =====

impl<'a> Buf<'a> for &'a mut ReadBuf {
    fn as_bytes(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    fn advance(&mut self, count: usize) {
        let count = count.min(ReadBuf::CAPACITY - self.off - self.len);
        self.ptr = unsafe { self.ptr.add(count) };
        self.len -= count;
        self.off += count;
    }
}

// ===== OS =====

impl ReadBuf {
    /// Returns `is_eof`.
    pub fn poll_read_fd<Fd: AsFd>(
        &mut self,
        fd: &Fd,
        cx: &mut Context<'_>,
    ) -> Poll<Result<bool, ReadError>> {
        match self.read_fd_sync(fd) {
            Ok(ok) => Poll::Ready(Ok(ok)),
            Err(err) => {
                if err.code().is_retry() {
                    cx.register_task_read();
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err))
                }
            }
        }
    }

    /// Returns `is_eof`.
    pub fn read_fd_sync<Fd: AsFd>(&mut self, fd: &Fd) -> Result<bool, ReadError> {
        let buf = unsafe {
            slice::from_raw_parts_mut(
                self.ptr.as_ptr().add(self.len).cast(),
                Self::CAPACITY - self.off - self.len,
            )
        };
        let read = io::read(fd, buf)?;
        self.len += read;
        Ok(read == 0)
    }
}

// ===== protocol =====

impl ReadBuf {
    pub fn read_msg(&mut self) -> Result<Option<Message<'_>>, DecodeError> {
        Message::read(self)
    }
}
