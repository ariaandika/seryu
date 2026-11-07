use core::{mem, ops, ptr};

use genos::{arch, sys};
use seryu::event::Epoll;

use crate::rt::Reactor;

#[repr(transparent)]
pub struct EpollHandle(mem::ManuallyDrop<Epoll>);

impl Clone for EpollHandle {
    fn clone(&self) -> Self {
        // SAFETY: `ManuallyDrop` prevent `Epoll` closing the fd on drop
        unsafe { ptr::read(self) }
    }
}

impl EpollHandle {
    pub fn new_static() -> Result<Self, sys::Error<arch::sys_epoll_create1>> {
        Epoll::new().map(|e| Self(mem::ManuallyDrop::new(e)))
    }
}

impl ops::Deref for EpollHandle {
    type Target = Epoll;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Reactor for EpollHandle {
    type Error = sys::Error<arch::sys_epoll_ctl>;

    fn add<Fd: genos::fd::AsFd, D: seryu::event::EventData>(
        &self,
        fd: &Fd,
        data: D,
    ) -> Result<(), Self::Error> {
        Epoll::add(self, fd, data)
    }

    fn delete<Fd: genos::fd::AsFd>(&self, fd: &Fd) -> Result<(), Self::Error> {
        Epoll::delete(self, fd)
    }
}
