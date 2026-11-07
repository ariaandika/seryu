use core::mem::MaybeUninit;
use core::slice;
use core::task::{Context, Poll};

use seryu::bytes::Buf;
use seryu::os::{ReadError, Socket};
use seryu::postgres::DecodeError;
use seryu::postgres::backend::Message;

use crate::alloc::{self, Mmap};

// ===== ReadBuf =====

pub struct ReadBuf {
    buf: Mmap<[MaybeUninit<u8>; Self::CAPACITY]>,
    len: usize,
    off: usize,
}

// ===== getters/setters =====

impl ReadBuf {
    pub const CAPACITY: usize = 1024;

    pub fn new() -> alloc::Result<ReadBuf> {
        Mmap::array().map(|buf| Self { buf, len: 0, off: 0 })
    }
}

// ===== impl Buf =====

impl<'a> Buf<'a> for &'a mut ReadBuf {
    fn as_bytes(&self) -> &'a [u8] {
        unsafe {
            slice::from_raw_parts(self.buf.as_ptr().add(self.off).cast(), self.len - self.off)
        }
    }

    fn advance(&mut self, count: usize) {
        self.off = self.len.min(self.off + count);
    }
}

// ===== OS =====

impl ReadBuf {
    /// Returns `is_eof`.
    pub fn poll_read_fd(
        &mut self,
        io: &Socket,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<bool, ReadError>> {
        match self.read_fd_sync(io) {
            Ok(ok) => Poll::Ready(Ok(ok)),
            Err(err) => {
                if err.code().is_retry() {
                    // cx.register_task_read();
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err))
                }
            }
        }
    }

    /// Returns `is_eof`.
    pub fn read_fd_sync(&mut self, io: &Socket) -> Result<bool, ReadError> {
        let buf = unsafe { self.buf.get_unchecked_mut(self.len..) };
        let Poll::Ready(read) = io.poll_read(buf)? else {
            unreachable!("socket is nonblocking");
        };
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
