use core::ptr::NonNull;
use core::task::{Context, Poll};

use genos::net::Socket;

use crate::error::Error;
use crate::http::Page;
use crate::http::read::ReadBuf;
use crate::http::request::Request;
use crate::http::write::{self, WriteBuf};
use crate::task::Task;

// ===== Handler =====

pub fn request_handler(request: Request) {
    println!(
        "{} {}",
        String::from_utf8_lossy(request.method()),
        String::from_utf8_lossy(request.target())
    );
    for header in request.headers() {
        println!("{header:?}");
    }
}

// ===== Task =====

pub type TaskPage = Page<2>;

pub struct ClientTask {
    // states
    phase: Phase,
    pub task: Task,

    // memory buffer
    read_buf: ReadBuf,
    write_buf: WriteBuf,

    // external resources
    pub page: NonNull<u8>,
    pub socket: Socket,
}

enum Phase {
    ReadRequest,
    WriteResponse,
    Disconnect,
}

impl ClientTask {
    pub fn new(socket: Socket, page: TaskPage) -> Self {
        let page_ptr = page.as_ptr();
        let [rd, wr] = page.split();
        let read_buf = ReadBuf::from_raw(rd);
        let write_buf = WriteBuf::from_raw(wr);
        Self {
            task: Task::RD,
            phase: Phase::ReadRequest,
            read_buf,
            write_buf,
            page: page_ptr,
            socket,
        }
    }
}

// ===== poll entry point =====

impl ClientTask {
    pub fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.poll_inner(cx).map(|res| {
            if res.is_err() {
                self.phase = Phase::Disconnect;
            }
        })
    }

    fn poll_inner(&mut self, _cx: &mut Context) -> Poll<Result<(), Error>> {
        loop {
            match self.phase {
                Phase::ReadRequest => {
                    match self.read_buf.parse_request()? {
                        Some(request) => {
                            request_handler(request);
                            write::response_writer(&mut self.write_buf)?;
                            self.phase = Phase::WriteResponse;
                        }
                        None => match self.read_buf.poll_read_fd(&self.socket)? {
                            Poll::Ready(is_eof) => {
                                if is_eof {
                                    self.phase = Phase::Disconnect;
                                }
                            }
                            Poll::Pending => {
                                // register read interest
                                self.task |= Task::RD;
                            }
                        },
                    }
                }
                Phase::WriteResponse => {
                    if self.write_buf.poll_flush(&self.socket)?.is_pending() {
                        // TODO: sync "write interest" with "epoll write interest"
                        self.task |= Task::WR;
                        return Poll::Pending;
                    }
                    self.phase = Phase::ReadRequest;
                }
                Phase::Disconnect => return Poll::Ready(Ok(())),
            }
        }
    }
}
