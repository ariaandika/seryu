use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll, ready};

use seryu::alloc::MmapAlloc;
use seryu::os::Socket;

use crate::alloc;
use crate::error::Error;
use crate::http::any::AnyFuture;
use crate::task::Interest;

// ===== Task =====

const _: () = assert!(size_of::<ClientTask>() == alloc::PAGE_SIZE * 2);

#[repr(C, align(4096))]
pub struct ClientTask {
    pub interest: Interest,
    pub socket: Socket,
    phase: Phase,
    read_buf: super::read::ReadBuf,
    write_buf: super::write::WriteBuf,
}

enum Phase {
    ReadRequest,
    Handler(AnyFuture<MmapAlloc>),
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

impl Future for ClientTask {
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = unsafe { self.get_unchecked_mut() };
        me.interest = me.interest.unset(Interest::QUEUE);

        loop {
            match &mut me.phase {
                Phase::ReadRequest => {
                    match me.read_buf.parse_request()? {
                        Some(req) => {
                            let fut = crate::request_handler(req);
                            me.phase = Phase::Handler(AnyFuture::new(fut, MmapAlloc)?);
                        }
                        None => match me.read_buf.poll_read(&me.socket)? {
                            Poll::Ready(is_eof) => {
                                if is_eof {
                                    me.phase = Phase::Disconnect;
                                }
                            }
                            Poll::Pending => {
                                // register read interest
                                me.interest |= Interest::READ;
                            }
                        },
                    }
                }
                Phase::Handler(fut) => {
                    ready!(Pin::new(fut).poll(cx));
                    super::write::response_writer(&mut me.write_buf)?;
                    me.phase = Phase::WriteResponse;
                }
                Phase::WriteResponse => {
                    if me.write_buf.poll_flush(&me.socket)?.is_pending() {
                        // TODO: sync "write interest" with "epoll write interest"
                        me.interest |= Interest::WRITE;
                        return Poll::Pending;
                    }
                    me.phase = Phase::ReadRequest;
                }
                Phase::Disconnect => return Poll::Ready(Ok(())),
            }
        }
    }
}
