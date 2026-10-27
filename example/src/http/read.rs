use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::slice;
use core::task::Poll;

use genos::fd::AsFd;
use genos::io;
use seryu::error::ReadError;
use seryu::h1;

use crate::error::Error;
use crate::http::Page;
use crate::http::request::{Request, RequestData};

// ===== ReadBuf =====

// #[size(4096)]
// struct MemoryLayout {
//     request: RequestData,
//     bytes: [u8],
// }

pub const CAP: usize = 4096;
pub const REQ_SIZE: usize = size_of::<RequestData>();
pub const BYTES_SIZE: usize = CAP - REQ_SIZE;

pub struct ReadBuf {
    buf: NonNull<[u8]>,
    off: usize,
}

impl ReadBuf {
    pub fn from_raw(page: Page) -> Self {
        let ptr = page.as_ptr();
        Self { buf: NonNull::slice_from_raw_parts(ptr, 0), off: 0 }
    }

    fn reqdata_mut(&mut self) -> &mut RequestData {
        unsafe { self.buf.cast().as_mut() }
    }

    fn parts(&mut self) -> (&mut MaybeUninit<RequestData>, &mut [u8]) {
        unsafe { (self.buf.cast().as_mut(), self.buf.byte_add(REQ_SIZE + self.off).as_mut()) }
    }

    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        // TODO: h1: request pipeline offset
        unsafe {
            let ptr = self
                .buf
                .as_ptr()
                .byte_add(REQ_SIZE + self.off + self.buf.len())
                .cast();
            slice::from_raw_parts_mut(ptr, BYTES_SIZE - self.off - self.buf.len())
        }
    }

    pub fn parse_request(&mut self) -> Result<Option<Request>, Error> {
        let (reqdata_mut, bytes) = self.parts();
        let Some(bytes) = read_request(bytes) else {
            return Ok(None);
        };
        RequestData::parse(bytes, reqdata_mut)?;
        self.off += bytes.len();
        Ok(Some(Request::new(self.reqdata_mut())))
    }
}

fn read_request(bytes: &mut [u8]) -> Option<&mut [u8]> {
    let base = bytes.as_mut_ptr();

    let mut bytes = &bytes[..];
    while !h1::read_line(&mut bytes)?.is_empty() {}

    let read = unsafe { bytes.as_ptr().offset_from_unsigned(base) };

    Some(unsafe { slice::from_raw_parts_mut(base, read) })
}

impl ReadBuf {
    /// Returns `is_eof`.
    pub fn poll_read_fd<Fd: AsFd>(&mut self, fd: &Fd) -> Poll<Result<bool, ReadError>> {
        match io::read(fd, self.spare_mut()) {
            Ok(read) => {
                self.buf = NonNull::slice_from_raw_parts(self.buf.cast(), self.buf.len() + read);
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
