use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::task::{Poll, ready};

use genos::io;
use genos::net::Socket;
use seryu::bytes::Writer;

use crate::Error;
use crate::alloc::Page;
use crate::http::h1::HeadersBuf;
use crate::http::{IoBuffer, TaskPage, h1};

// ===== Task =====

pub struct Task {
    pub page: TaskPage,
    pub socket: Socket,
    pub is_queued: bool,
    read_off: usize,
    read_buf: IoBuffer,
    write_buf: IoBuffer,
    headers: NonNull<HeadersBuf<'static>>,
    phase: Phase,
}

enum Phase {
    Request,
    Response,
}

impl Task {
    pub fn new(socket: Socket, page: TaskPage) -> Result<Self, Error> {
        let base = page.as_ptr();
        let read_buf = unsafe { IoBuffer::from_raw(base) };
        let write_buf = unsafe { IoBuffer::from_raw(base.add(Page::SIZE)) };
        let headers = unsafe { base.add(Page::SIZE * 2).cast() };
        Ok(Self {
            page,
            socket,
            is_queued: false,
            read_off: 0,
            read_buf,
            write_buf,
            headers,
            phase: Phase::Request,
        })
    }

    pub fn poll(&mut self) -> Poll<()> {
        self.poll_inner().map(drop)
    }
}

impl Task {
    fn poll_inner(&mut self) -> Poll<Result<(), Error>> {
        for _ in 0..24 {
            match self.phase {
                Phase::Request => {
                    let len = loop {
                        match h1::scan_request(&self.read_buf.as_slice()[self.read_off..])? {
                            0 => {
                                ready!(self.poll_read())?;
                                continue;
                            }
                            len => break len as usize,
                        }
                    };
                    let request = &self.read_buf.as_slice()[self.read_off..][..len];
                    let mut reqline = MaybeUninit::uninit();
                    let headers = unsafe { self.headers.cast().as_mut() };
                    let header_len = h1::parse_request(request, &mut reqline, headers)?;

                    unsafe {
                        let headers = headers[..header_len as usize].assume_init_ref();
                        h1::request_handler(reqline.assume_init_ref(), headers)
                    };
                    self.read_off += len;
                    self.phase = Phase::Response;
                }
                Phase::Response => {
                    let mut writer = Writer::new(self.write_buf.spare_capacity_mut());
                    h1::response_writer(&mut writer)?;
                    io::write(&self.socket, writer.init())?;
                    self.write_buf.clear();
                    self.phase = Phase::Request;
                }
            }
        }
        Poll::Pending
    }

    fn poll_read(&mut self) -> Poll<Result<(), Error>> {
        let read = match io::read(&self.socket, self.read_buf.spare_capacity_mut()) {
            Ok(0) => return Poll::Ready(Ok(())),
            Ok(ok) => ok,
            Err(err) => {
                return if err.code().is_retry() {
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err.into()))
                };
            }
        };
        let new_len = self.read_buf.len() + read;
        unsafe { self.read_buf.set_len(new_len) };
        Poll::Ready(Ok(()))
    }
}
