use core::ptr::NonNull;
use core::slice;
use core::task::{Context, Poll, ready};
use std::mem::MaybeUninit;

use genos::fd::AsFd;
use genos::{arch, sys};
use seryu::bytes::BufMut;
use seryu::h1;

use crate::alloc::Page;
use crate::{Error, io};

// ===== ReadBuf =====

pub const CAP: usize = Page::SIZE;

pub struct WriteBuf {
    ptr: NonNull<u8>,
    rem: usize,
    off: usize,
}

impl WriteBuf {
    pub unsafe fn from_raw(ptr: NonNull<u8>) -> Self {
        Self { ptr, rem: CAP, off: 0 }
    }

    fn spare(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr().cast(), self.rem) }
    }

    unsafe fn assume_init_len(&mut self, len: usize) {
        debug_assert_eq!(self.off, 0, "WriterBuf must not be written when flushing");
        self.ptr = unsafe { self.ptr.add(len) };
        self.rem -= len;
    }

    pub fn clear(&mut self) {
        self.ptr = unsafe { self.ptr.sub(CAP - self.rem) };
        self.rem = CAP;
        self.off = 0;
    }

    pub fn poll_flush<Fd: AsFd>(
        &mut self,
        fd: &Fd,
        cx: &mut Context,
    ) -> Poll<Result<(), sys::Error<arch::sys_write>>> {
        let base = unsafe { self.ptr.as_ptr().sub(CAP - self.rem) };
        while let len = CAP - self.rem - self.off
            && len != 0
        {
            let buf = unsafe { slice::from_raw_parts(base, len) };
            self.off += ready!(io::poll_write(fd, buf, cx))?;
        }
        Poll::Ready(Ok(()))
    }
}

impl BufMut for &mut WriteBuf {
    #[inline]
    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self.spare()
    }

    #[inline]
    unsafe fn advance_mut(&mut self, count: usize) {
        unsafe { self.assume_init_len(count) };
    }
}

// ===== h1 =====

pub fn response_writer(write_buf: &mut WriteBuf) -> Result<(), Error> {
    h1::StatusLine { version: b"HTTP/1.1", status: b"200", reason: b"OK" }
        .serialize(&mut *write_buf)?;
    h1::Field::new(b"Content-Length", b"0").serialize(&mut *write_buf)?;
    h1::Field::serialize_eoh(&mut *write_buf)?;
    Ok(())
}
