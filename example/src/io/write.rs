use core::task::{Context, Poll};

use genos::fd::AsFd;
use genos::{arch, io, sys};

use crate::task::WakerData;

pub fn poll_write<Fd: AsFd + ?Sized>(
    fd: &Fd,
    buf: &[u8],
    cx: &mut Context,
) -> Poll<Result<usize, sys::Error<arch::sys_write>>> {
    match io::write(fd, buf) {
        Ok(read) => Poll::Ready(Ok(read)),
        Err(err) => {
            if err.code().is_retry() {
                WakerData::from_waker(cx.waker()).register_write(fd);
                Poll::Pending
            } else {
                Poll::Ready(Err(err))
            }
        }
    }
}
