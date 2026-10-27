use core::mem::{self, MaybeUninit};
use core::slice;

use genos::event::epoll::{self, Epoll};
use genos::fd::{AsFd, FromRawFd};
use genos::{arch, sys};

// ===== Event =====

#[repr(transparent)]
pub struct Event(epoll::Event);

impl Event {
    /// Returns `true` if event has read interest.
    pub fn has_read(&self) -> bool {
        self.0.events & Epoll::IN == Epoll::IN
    }

    /// Returns `true` if event has write interest.
    pub fn has_write(&self) -> bool {
        self.0.events & Epoll::OUT == Epoll::OUT
    }

    /// Returns event identifier.
    pub fn data<D: EventData>(&self) -> D {
        D::from_data(self.0.data)
    }
}

// ===== EventData =====

/// Application event identifier.
pub trait EventData: Sized {
    fn from_data(data: u64) -> Self;

    fn into_data(self) -> u64;
}

impl EventData for u64 {
    fn from_data(data: u64) -> Self {
        data
    }

    fn into_data(self) -> u64 {
        self
    }
}

// ===== Polling =====

pub struct Polling(Epoll);

impl Polling {
    pub fn new() -> Result<Self, sys::Error<arch::sys_epoll_create1>> {
        Epoll::create(Epoll::CLOEXEC).map(Self)
    }

    pub fn handle(&self) -> PollHandle {
        PollHandle::new(self)
    }

    pub fn add<Fd: AsFd, D: EventData>(
        &self,
        fd: &Fd,
        data: D,
    ) -> Result<(), sys::Error<arch::sys_epoll_ctl>> {
        self.0.add(fd, Epoll::IN | Epoll::ET, data.into_data())
    }

    pub fn delete<Fd: AsFd>(&self, fd: &Fd) -> Result<(), sys::Error<arch::sys_epoll_ctl>> {
        self.0.delete(fd)
    }

    pub fn wait<'a>(
        &self,
        buf: &'a mut [MaybeUninit<epoll::Event>],
    ) -> Result<&'a [Event], sys::Error<arch::sys_epoll_wait>> {
        let len = self.0.wait(buf, -1)?;
        // SAFETY: `Event` is transparent over epoll event, `len` returned from os
        Ok(unsafe { slice::from_raw_parts(buf.as_ptr().cast(), len) })
    }
}

// ===== PollHandle =====

pub struct PollHandle(mem::ManuallyDrop<Polling>);

impl PollHandle {
    fn new(poll: &Polling) -> Self {
        let fd = unsafe { <_>::from_raw_fd(poll.0.as_raw_fd()) };
        Self(mem::ManuallyDrop::new(Polling(fd)))
    }
}

impl Clone for PollHandle {
    fn clone(&self) -> Self {
        Self::new(&self.0)
    }
}

impl core::ops::Deref for PollHandle {
    type Target = Polling;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
