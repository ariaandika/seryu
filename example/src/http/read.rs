use core::mem::MaybeUninit;
use core::slice;
use core::task::Poll;

use genos::fd::AsFd;
use genos::io;
use seryu::error::ReadError;
use seryu::h1;

use crate::alloc;
use crate::error::Error;
use crate::http::request::{Request, RequestData};

const SIZE: usize = alloc::PAGE_SIZE;
const META: usize = size_of::<usize>() * 2 + size_of::<RequestData>();
const BYTES: usize = SIZE - META;

const _: () = assert!(size_of::<ReadBuf>() == SIZE);

// ===== ReadBuf =====

#[repr(C)]
pub struct ReadBuf {
    off: usize,
    len: usize,
    req: MaybeUninit<RequestData>,
    buf: [MaybeUninit<u8>; BYTES],
}

impl ReadBuf {
    pub fn clear(&mut self) {
        self.off = 0;
        self.len = 0;
    }

    pub fn parse_request(&mut self) -> Result<Option<Request>, Error> {
        let bytes = unsafe {
            self.buf
                .get_unchecked_mut(self.off..self.len)
                .assume_init_mut()
        };
        let Some(bytes) = scan_request(bytes) else {
            return Ok(None);
        };
        RequestData::parse(bytes, &mut self.req)?;
        self.off += bytes.len();
        Ok(Some(Request::new(unsafe { self.req.assume_init_mut() })))
    }

    /// Returns `is_eof`.
    pub fn poll_read<Fd: AsFd>(&mut self, fd: &Fd) -> Poll<Result<bool, ReadError>> {
        let buf = unsafe { self.buf.get_unchecked_mut(self.len..) };
        match io::read(fd, buf) {
            Ok(read) => {
                self.len += read;
                Poll::Ready(Ok(read == 0))
            }
            Err(err) => {
                if err.code().is_retry() {
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err))
                }
            }
        }
    }
}

// there is better way for only scanning request
fn scan_request(bytes: &mut [u8]) -> Option<&mut [u8]> {
    let base = bytes.as_mut_ptr();

    let mut bytes = &bytes[..];
    while !h1::read_line(&mut bytes)?.is_empty() {}

    unsafe {
        let read = bytes.as_ptr().offset_from_unsigned(base);
        Some(slice::from_raw_parts_mut(base, read))
    }
}
