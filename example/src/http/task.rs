use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::task::{Context, Poll, ready};

use genos::io;
use genos::net::Socket;
use seryu::bytes::Writer;

use crate::Error;
use crate::alloc::Page;
use crate::http::h1::HeadersBuf;
use crate::http::{IoBuffer, TaskPage, h1};
use crate::task::WakerData;

// ===== Task =====

pub struct HttpTask {
    pub page: TaskPage,
    pub socket: Socket,
    read_off: usize,
    read_buf: IoBuffer,
    write_buf: IoBuffer,
    headers: NonNull<HeadersBuf<'static>>,
    phase: Phase,
}

enum Phase {
    Request,
    Response,
    Disconnect,
}

impl HttpTask {
    pub fn new(socket: Socket, page: TaskPage) -> Result<Self, Error> {
        let base = page.as_ptr();
        let read_buf = unsafe { IoBuffer::from_raw(base) };
        let write_buf = unsafe { IoBuffer::from_raw(base.add(Page::SIZE)) };
        let headers = unsafe { base.add(Page::SIZE * 2).cast() };
        Ok(Self { page, socket, read_off: 0, read_buf, write_buf, headers, phase: Phase::Request })
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
                Phase::Request => {
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
                    self.phase = Phase::Response;
                }
                Phase::Response => {
                    let mut writer = Writer::new(self.write_buf.spare_capacity_mut());
                    h1::response_writer(&mut writer)?;
                    io::write(&self.socket, writer.init())?;
                    self.write_buf.clear();
                    self.phase = Phase::Request;
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
        match io::read(&self.socket, self.read_buf.spare_capacity_mut()) {
            Ok(read) => {
                if read == 0 {
                    // read `0` means EOF, therefore disconnect
                    self.phase = Phase::Disconnect;
                } else {
                    let new_len = self.read_buf.len() + read;
                    unsafe { self.read_buf.set_len(new_len) };
                }
                Poll::Ready(Ok(()))
            }
            Err(err) => {
                if err.code().is_retry() {
                    // would block, register the event interest
                    WakerData::from_waker(cx.waker()).register_read(&self.socket);
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err.into()))
                }
            }
        }
    }
}
