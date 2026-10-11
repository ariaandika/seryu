use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::task::{Context, Poll, ready};

use genos::net::Socket;
use seryu::bytes::Writer;

use crate::alloc::Page;
use crate::http::h1::HeadersBuf;
use crate::http::{IoBuffer, TaskPage, h1};
use crate::{Error, io};

// ===== Task =====

pub struct HttpTask {
    pub page: TaskPage,
    pub socket: Socket,
    read_off: usize,
    read_buf: IoBuffer,
    write_off: usize,
    write_buf: IoBuffer,
    headers: NonNull<HeadersBuf<'static>>,
    phase: Phase,
}

enum Phase {
    ReadRequest,
    WriteResponse,
    Disconnect,
}

impl HttpTask {
    pub fn new(socket: Socket, page: TaskPage) -> Result<Self, Error> {
        let base = page.as_ptr();
        let read_buf = unsafe { IoBuffer::from_raw(base) };
        let write_buf = unsafe { IoBuffer::from_raw(base.add(Page::SIZE)) };
        let headers = unsafe { base.add(Page::SIZE * 2).cast() };
        Ok(Self {
            page,
            socket,
            read_off: 0,
            read_buf,
            write_off: 0,
            write_buf,
            headers,
            phase: Phase::ReadRequest,
        })
    }
}

// ===== poll entry point =====

impl HttpTask {
    pub fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.poll_inner(cx).map(drop)
    }

    fn poll_inner(&mut self, cx: &mut Context) -> Poll<Result<(), Error>> {
        loop {
            match self.phase {
                Phase::ReadRequest => {
                    let len = match h1::scan_request(&self.read_buf.as_slice()[self.read_off..])? {
                        0 => {
                            ready!(self.poll_read(cx))?;
                            continue;
                        }
                        len => len as usize,
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

                    // response
                    let mut writer = Writer::new(self.write_buf.spare_capacity_mut());
                    h1::response_writer(&mut writer)?;
                    self.write_off = 0;
                    self.phase = Phase::WriteResponse;
                }
                Phase::WriteResponse => {
                    ready!(self.poll_write_all(cx)?);
                    self.write_off = 0;
                    self.write_buf.clear();
                }
                Phase::Disconnect => {
                    return Poll::Ready(Ok(()));
                }
            }
        }
    }
}

// ===== poll underlying io =====

impl HttpTask {
    fn poll_read(&mut self, cx: &mut Context) -> Poll<Result<(), Error>> {
        let read = ready!(io::poll_read(&self.socket, self.read_buf.spare_capacity_mut(), cx))?;
        if read == 0 {
            // read `0` means EOF, therefore disconnect
            self.phase = Phase::Disconnect;
        } else {
            let new_len = self.read_buf.len() + read;
            unsafe { self.read_buf.set_len(new_len) };
        }
        Poll::Ready(Ok(()))
    }

    fn poll_write_all(&mut self, cx: &mut Context) -> Poll<Result<(), Error>> {
        while self.write_off < self.write_buf.len() {
            let buf = &self.write_buf.as_slice()[self.write_off..];
            self.write_off += ready!(io::poll_write(&self.socket, buf, cx))?;
        }
        Poll::Ready(Ok(()))
    }
}
