use core::task::{Context, Poll};

use genos::fd::AsFd;
use genos::io;
use seryu::error::WriteError;

use crate::task::ContextExt;

pub fn poll_write<Fd: AsFd + ?Sized>(
    fd: &Fd,
    buf: &[u8],
    cx: &mut Context,
) -> Poll<Result<usize, WriteError>> {
    match io::write(fd, buf) {
        Ok(read) => Poll::Ready(Ok(read)),
        Err(err) => {
            if err.code().is_retry() {
                cx.register_write(fd);
                Poll::Pending
            } else {
                Poll::Ready(Err(err))
            }
        }
    }
}
