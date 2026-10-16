use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::task::{Context, Poll, ready};
use core::{fmt, slice};

use genos::fd::AsFd;
use genos::{arch, sys};
use seryu::bytes::Reader;
use seryu::h1;

use crate::alloc::Page;
use crate::{Error, io};

// ===== ReadBuf =====

pub const CAP: usize = Page::SIZE;

pub struct ReadBuf {
    ptr: NonNull<u8>,
    len: usize,
    off: usize,
}

impl ReadBuf {
    pub unsafe fn from_raw(ptr: NonNull<u8>) -> Self {
        Self { ptr, len: 0, off: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl ReadBuf {
    /// Returns `is_eof`.
    pub fn poll_read_fd<Fd: AsFd>(
        &mut self,
        fd: &Fd,
        cx: &mut Context<'_>,
    ) -> Poll<Result<bool, sys::Error<arch::sys_read>>> {
        let buf = unsafe {
            slice::from_raw_parts_mut(
                self.ptr.as_ptr().add(self.len).cast(),
                CAP - self.off - self.len,
            )
        };
        let read = ready!(io::poll_read(fd, buf, cx))?;
        self.len += read;
        Poll::Ready(Ok(read == 0))
    }

    pub fn read_request(&mut self) -> Option<&[u8]> {
        unsafe {
            let mut bytes = slice::from_raw_parts(self.ptr.as_ptr(), self.len);
            while !h1::read_line(&mut bytes)?.is_empty() {}
            let read = bytes.as_ptr().offset_from_unsigned(self.ptr.as_ptr());
            let line = slice::from_raw_parts(self.ptr.as_ptr(), read);
            self.ptr = self.ptr.add(read);
            self.len -= read;
            self.off += read;
            Some(line)
        }
    }
}

// ===== h1 =====

pub type HeadersBuf<'a> = [MaybeUninit<h1::Field<'a>>; MAX_HEADERS as usize];

const MAX_HEADERS: u32 = 24;

pub fn parse_request<'a, 'h>(
    bytes: &'a [u8],
    reqline: &mut MaybeUninit<h1::RequestLine<'a>>,
    headers: &'h mut HeadersBuf<'a>,
) -> Result<&'h [h1::Field<'a>], Error> {
    let mut reader = Reader::new(bytes);
    let Some(line) = reader.read_with(h1::read_line) else {
        return Err(h1::ParseError::InsufficientBytes.into());
    };
    h1::parse_reqline(line, reqline)?;
    let mut header_len = 0;
    loop {
        let Some(line) = reader.read_with(h1::read_line) else {
            return Err(h1::ParseError::InsufficientBytes.into());
        };
        if line.is_empty() {
            break;
        }
        let Some(header_mut) = headers.get_mut(header_len) else {
            return Err(ExcessiveHeaders.into());
        };
        h1::parse_field(line, header_mut)?;
        header_len += 1;
    }
    Ok(unsafe { headers.get_unchecked(..header_len).assume_init_ref() })
}

pub struct ExcessiveHeaders;

impl fmt::Display for ExcessiveHeaders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "excessive headers")
    }
}
