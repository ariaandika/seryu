use core::mem::MaybeUninit;
use core::task::{Context, Poll};

use genos::fd::AsFd;
use genos::io;
use seryu::alloc::OutOfCapacity;
use seryu::bytes::BufMut;
use seryu::error::WriteError;
use seryu::postgres::Encode;

use crate::alloc::{self, Mmap};
use crate::task::ContextExt;

// ===== WriteBuf =====

pub struct WriteBuf {
    buf: Mmap<[MaybeUninit<u8>; Self::CAPACITY]>,
    len: usize,
    off: usize,
}

// ===== getters/setters =====

impl WriteBuf {
    pub const CAPACITY: usize = 1024;

    pub fn new() -> alloc::Result<WriteBuf> {
        Mmap::array().map(|buf| Self { buf, len: 0, off: 0 })
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.off = 0;
    }
}

// ===== impl BufMut =====

impl BufMut for &mut WriteBuf {
    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe { self.buf.get_unchecked_mut(self.len..) }
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
        let buf = unsafe { self.buf.get_unchecked_mut(..self.len).assume_init_ref() };
        while let Some(bytes) = buf.get(self.off..)
            && !bytes.is_empty()
        {
            self.off += io::write(fd, bytes)?;
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
