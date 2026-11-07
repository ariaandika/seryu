use core::mem::MaybeUninit;
use core::task::{Poll, ready};

use genos::sys;
use seryu::bytes::BufMut;
use seryu::h1;
use seryu::os::{Socket, WriteError};

use crate::alloc;
use crate::error::Error;

const SIZE: usize = alloc::PAGE_SIZE / 2;
const META: usize = size_of::<usize>() * 2;
const BYTES: usize = SIZE - META;

const _: () = assert!(size_of::<WriteBuf>() == SIZE);

// ===== ReadBuf =====

#[repr(C)]
pub struct WriteBuf {
    off: usize,
    len: usize,
    buf: [MaybeUninit<u8>; BYTES],
}

impl WriteBuf {
    pub fn clear(&mut self) {
        self.off = 0;
        self.len = 0;
    }

    pub fn poll_flush(&mut self, io: &Socket) -> Poll<Result<(), WriteError>> {
        let bytes = unsafe { self.buf.get_unchecked_mut(..self.len).assume_init_ref() };
        while let Some(bytes) = bytes.get(self.off..)
            && !bytes.is_empty()
        {
            let write = ready!(io.poll_write(bytes))?;
            if write == 0 {
                return Poll::Ready(Err(WriteError::from(sys::ErrCode::ECONNABORTED)));
            }
            self.off += write
        }
        self.clear();
        Poll::Ready(Ok(()))
    }
}

impl BufMut for &mut WriteBuf {
    #[inline]
    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe { self.buf.get_unchecked_mut(self.len..) }
    }

    #[inline]
    unsafe fn advance_mut(&mut self, count: usize) {
        self.len += count;
    }
}

pub fn response_writer(write_buf: &mut WriteBuf) -> Result<(), Error> {
    h1::StatusLine { version: b"HTTP/1.1", status: b"200", reason: b"OK" }
        .serialize(&mut *write_buf)?;
    h1::Field::new(b"Content-Length", b"0").serialize(&mut *write_buf)?;
    h1::Field::serialize_eoh(&mut *write_buf)?;
    Ok(())
}
