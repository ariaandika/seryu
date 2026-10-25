use core::task::{Context, Poll, ready};

use genos::net::Socket;

use crate::alloc::Page;
use crate::error::Error;
use crate::http::read::ReadBuf;
use crate::http::request::Request;
use crate::http::write::{self, WriteBuf};
use crate::task::{Task, WithTask};

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

pub type TaskPage = Page<4>;

pub struct HttpTask {
    // states
    phase: Phase,
    task: Task,

    // memory buffer
    read_buf: ReadBuf,
    write_buf: WriteBuf,

    // external resources
    pub page: TaskPage,
    pub socket: Socket,
    // req_bytes: [u8],
    // res_bytes: [u8],
}

enum Phase {
    ReadRequest,
    WriteResponse,
    Disconnect,
}

impl HttpTask {
    pub fn new(socket: Socket, page: TaskPage) -> Self {
        let base = page.as_ptr();
        let read_buf = unsafe { ReadBuf::from_raw(base) };
        let write_buf = unsafe { WriteBuf::from_raw(base.add(Page::SIZE)) };
        Self { task: Task::RD, phase: Phase::ReadRequest, read_buf, write_buf, page, socket }
    }
}

impl WithTask for HttpTask {
    fn state(&self) -> Task {
        self.task
    }

    fn set_state(&mut self, state: Task) {
        self.task = state;
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
                    let Some(request) = self.read_buf.parse_request()? else {
                        if ready!(self.read_buf.poll_read_fd(&self.socket, cx))? {
                            self.phase = Phase::Disconnect;
                        }
                        continue;
                    };

                    request_handler(request);

                    // response

                    write::response_writer(&mut self.write_buf)?;
                    self.phase = Phase::WriteResponse;
                }
                Phase::WriteResponse => {
                    ready!(self.write_buf.poll_flush(&self.socket, cx)?);
                    self.write_buf.clear();
                    self.phase = Phase::ReadRequest;
                }
                Phase::Disconnect => return Poll::Ready(Ok(())),
            }
        }
    }
}
