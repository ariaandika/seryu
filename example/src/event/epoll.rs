use core::mem::MaybeUninit;

use genos::event::epoll::{self, Epoll};
use genos::fd::AsFd;
use genos::{arch, sys};
use seryu::alloc::Allocator;

use crate::alloc::{self, Owned};

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

// ===== EventBuf =====

pub const EVENT_CAP: usize = alloc::PAGE_SIZE / 2 / size_of::<Event>();

pub type EventBuf<A> = Owned<[MaybeUninit<Event>; EVENT_CAP], A>;

impl Polling {
    pub fn event_buf<A: Allocator>(alloc: A) -> Result<EventBuf<A>, A::Error> {
        Owned::uninit_array(alloc)
    }
}
