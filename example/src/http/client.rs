use core::mem::MaybeUninit;
use core::task::{Context, Poll};

use genos::net::Socket;

use crate::alloc;
use crate::error::Error;
use crate::http::request::Request;
use crate::task::Interest;

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

const _: () = assert!(size_of::<ClientTask>() == alloc::PAGE_SIZE * 2);

#[repr(C, align(4096))]
pub struct ClientTask {
    phase: Phase,
    pub interest: Interest,
    pub socket: Socket,
    read_buf: super::read::ReadBuf,
    write_buf: super::write::WriteBuf,
}

enum Phase {
    ReadRequest,
    Handler,
    WriteResponse,
    Disconnect,
}

impl ClientTask {
    pub(super) fn initialize(socket: Socket, me: &mut MaybeUninit<Self>) {
        unsafe {
            let me = me.as_mut_ptr();
            debug_assert!(me.is_aligned());
            (&raw mut (*me).phase).write(Phase::ReadRequest);
            (&raw mut (*me).interest).write(Interest::READ);
            (&raw mut (*me).socket).write(socket);
            (*me).read_buf.clear();
            (*me).write_buf.clear();
        }
    }
}

// ===== poll entry point =====

impl ClientTask {
    pub fn poll(&mut self, _cx: &mut Context) -> Poll<Result<(), Error>> {
        loop {
            match self.phase {
                Phase::ReadRequest => {
                    match self.read_buf.parse_request()? {
                        Some(_) => {
                            self.phase = Phase::Handler;
                        }
                        None => match self.read_buf.poll_read(&self.socket)? {
                            Poll::Ready(is_eof) => {
                                if is_eof {
                                    self.phase = Phase::Disconnect;
                                }
                            }
                            Poll::Pending => {
                                // register read interest
                                self.interest |= Interest::READ;
                            }
                        },
                    }
                }
                Phase::Handler => {
                    let request = unsafe { self.read_buf.request_init() };
                    request_handler(request);
                    super::write::response_writer(&mut self.write_buf)?;
                    self.phase = Phase::WriteResponse;
                }
                Phase::WriteResponse => {
                    if self.write_buf.poll_flush(&self.socket)?.is_pending() {
                        // TODO: sync "write interest" with "epoll write interest"
                        self.interest |= Interest::WRITE;
                        return Poll::Pending;
                    }
                    self.phase = Phase::ReadRequest;
                }
                Phase::Disconnect => return Poll::Ready(Ok(())),
            }
        }
    }
}
