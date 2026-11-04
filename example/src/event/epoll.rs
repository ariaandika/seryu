use core::mem::MaybeUninit;

use genos::event::epoll::{self, Epoll};
use genos::fd::AsFd;
use genos::{arch, sys};

use crate::alloc::{self, Mmap};

// ===== EventBuf =====

pub const EVENT_CAP: usize = alloc::PAGE_SIZE / size_of::<Event>();

pub type EventBuf = Mmap<[MaybeUninit<Event>; EVENT_CAP]>;

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

    pub fn event_buf() -> alloc::Result<EventBuf> {
        Mmap::array()
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
        buf: &'a mut [MaybeUninit<Event>],
    ) -> Result<&'a [Event], sys::Error<arch::sys_epoll_wait>> {
        // SAFETY: `Event` is transparent over epoll event
        let ebuf = unsafe { &mut *(buf as *mut _ as *mut _) };
        let len = self.0.wait(ebuf, -1)?;
        // SAFETY: `len` represent initialized elements
        Ok(unsafe { buf.assume_init_mut().get_unchecked_mut(..len) })
    }
}
