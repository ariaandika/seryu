use core::mem::MaybeUninit;

use genos::event::Epoll as epoll;
use genos::fd::{AsFd, BorrowedFd};
use genos::{arch, sys};

use crate::event::{Event, EventData, Interest};

// ===== Epoll =====

/// Linux I/O event notification.
#[derive(Debug)]
#[repr(transparent)]
pub struct Epoll(epoll);

impl AsFd for Epoll {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Epoll {
    #[inline]
    pub fn new() -> Result<Self, sys::Error<arch::sys_epoll_create1>> {
        epoll::create(epoll::CLOEXEC).map(Self)
    }

    #[inline]
    pub fn add<Fd, D>(&self, fd: &Fd, data: D) -> Result<(), sys::Error<arch::sys_epoll_ctl>>
    where
        Fd: AsFd,
        D: EventData,
    {
        self.0.add(fd, epoll::IN | epoll::ET, data.into_raw())
    }

    #[inline]
    pub fn modify<Fd: AsFd, D: EventData>(
        &self,
        fd: &Fd,
        ev: Interest,
        data: D,
    ) -> Result<(), sys::Error<arch::sys_epoll_ctl>> {
        self.0.modify(fd, ev.raw() | epoll::ET, data.into_raw())
    }

    #[inline]
    pub fn delete<Fd: AsFd>(&self, fd: &Fd) -> Result<(), sys::Error<arch::sys_epoll_ctl>> {
        self.0.delete(fd)
    }

    #[inline]
    pub fn wait<'a>(
        &self,
        buf: &'a mut [MaybeUninit<Event>],
    ) -> Result<&'a mut [Event], sys::Error<arch::sys_epoll_wait>> {
        let len = self.0.wait(Event::as_raw_slice(buf), -1)?;
        // SAFETY: `len` represent initialized elements
        Ok(unsafe { buf.get_unchecked_mut(..len).assume_init_mut() })
    }
}
