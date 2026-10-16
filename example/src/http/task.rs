use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::task::{Context, Poll, ready};

use genos::net::Socket;
use seryu::h1;

use crate::Error;
use crate::alloc::Page;
use crate::http::read::{self, ReadBuf};
use crate::http::write::{self, WriteBuf};

// ===== Handler =====

pub fn request_handler(reqline: &h1::RequestLine, headers: &[h1::Field]) {
    println!("{reqline:?}");
    for header in headers {
        println!("{header:?}");
    }
}

// ===== Task =====

pub type HeadersBuf<'a> = [MaybeUninit<h1::Field<'a>>; MAX_HEADERS as usize];
pub type TaskPage = Page<4>;

const MAX_HEADERS: u32 = 24;
const MAX_HEADERS_SIZE: usize = 4096;

pub struct HttpTask {
    pub page: TaskPage,
    pub socket: Socket,
    read_buf: ReadBuf,
    write_buf: WriteBuf,
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
        let read_buf = unsafe { ReadBuf::from_raw(base) };
        let write_buf = unsafe { WriteBuf::from_raw(base.add(Page::SIZE)) };
        let headers = unsafe { base.add(Page::SIZE * 2).cast() };
        Ok(Self { page, socket, read_buf, write_buf, headers, phase: Phase::ReadRequest })
    }
}

// ===== poll entry point =====

impl HttpTask {
    pub fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.poll_inner(cx).map(|res| {
            if res.is_err() {
                self.phase = Phase::Disconnect;
            }
        })
    }

    fn poll_inner(&mut self, cx: &mut Context) -> Poll<Result<(), Error>> {
        loop {
            match self.phase {
                Phase::ReadRequest => {
                    let Some(request) = self.read_buf.read_request() else {
                        let is_eof = ready!(self.read_buf.poll_read_fd(&self.socket, cx))?;
                        if is_eof {
                            self.phase = Phase::Disconnect;
                        }
                        if self.read_buf.len() > MAX_HEADERS_SIZE {
                            return Poll::Ready(Err("excessive headers".into()));
                        }
                        continue;
                    };
                    let mut reqline = MaybeUninit::uninit();
                    let headers = unsafe { self.headers.cast().as_mut() };
                    let headers = read::parse_request(request, &mut reqline, headers)?;
                    let reqline = unsafe { reqline.assume_init_ref() };
                    request_handler(reqline, headers);

                    // response

                    write::response_writer(&mut self.write_buf)?;
                    self.phase = Phase::WriteResponse;
                }
                Phase::WriteResponse => {
                    ready!(self.write_buf.poll_flush(&self.socket, cx)?);
                    self.write_buf.clear();
                    self.phase = Phase::ReadRequest;
                }
                Phase::Disconnect => {
                    return Poll::Ready(Ok(()));
                }
            }
        }
    }
}
