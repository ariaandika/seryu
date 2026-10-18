use core::mem::MaybeUninit;
use core::task::{Context, Poll};

use genos::fd::AsFd;
use genos::{arch, io, sys};

use crate::task::ContextExt;

pub fn poll_read<'a, Fd: AsFd + ?Sized>(
    fd: &'a Fd,
    buf: &'a mut [MaybeUninit<u8>],
    cx: &mut Context,
) -> Poll<Result<usize, sys::Error<arch::sys_read>>> {
    match io::read(fd, buf) {
        Ok(read) => Poll::Ready(Ok(read)),
        Err(err) => {
            if err.code().is_retry() {
                cx.register_read(fd);
                Poll::Pending
            } else {
                Poll::Ready(Err(err))
            }
        }
    }
}
