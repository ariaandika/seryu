use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::slice;
use core::task::{Context, Poll, ready};

use genos::fd::AsFd;
use genos::{arch, sys};
use seryu::alloc::OutOfCapacity;
use seryu::bytes::BufMut;

use crate::io;

pub struct WriteBuf {
    ptr: NonNull<u8>,
    len: usize,
    off: usize,
}

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

impl BufMut for &mut WriteBuf {
    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe {
            let spare = WriteBuf::CAPACITY - self.off - self.len;
            slice::from_raw_parts_mut(self.ptr.as_ptr().cast(), spare)
        }
    }

    unsafe fn advance_mut(&mut self, count: usize) {
        self.len += count;
    }
}

impl WriteBuf {
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), OutOfCapacity> {
        let new_len = self.len + bytes.len();
        if new_len > Self::CAPACITY {
            return Err(OutOfCapacity);
        }
        unsafe {
            self.ptr
                .as_ptr()
                .copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            self.ptr = self.ptr.add(bytes.len());
        }
        self.len += bytes.len();
        Ok(())
    }

    /// Flush buffer.
    pub fn poll_flush<Fd: AsFd>(
        &mut self,
        fd: &Fd,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), sys::Error<arch::sys_write>>> {
        while self.len != 0 {
            let bytes = unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) };
            let write = ready!(io::poll_write(fd, bytes, cx))?;
            self.ptr = unsafe { self.ptr.add(write) };
            self.len -= write;
            self.off += write;
        }
        Poll::Ready(Ok(()))
    }
}
