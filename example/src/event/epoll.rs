use core::mem::MaybeUninit;
use core::{marker, slice};

use genos::event::epoll::{self, Epoll};
use genos::fd::AsFd;
use genos::{arch, sys};

// ===== EventData =====

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

// ===== Events =====

pub struct Events<'a, T> {
    events: slice::IterMut<'a, epoll::Event>,
    _p: marker::PhantomData<fn() -> T>,
}

impl<'a, T> Events<'a, T> {
    pub fn new(events: &'a mut [epoll::Event]) -> Self {
        Self { events: events.iter_mut(), _p: marker::PhantomData }
    }
}

impl<'a, T: super::EventData> Iterator for Events<'a, T> {
    type Item = (epoll::EventType, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.events.next().map(|e| (e.events, T::from_data(e.data)))
    }
}

// ===== Epoll =====

pub struct EventSource(Epoll);

impl EventSource {
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

    pub fn wait<'a, T: EventData>(
        &self,
        buf: &'a mut [MaybeUninit<epoll::Event>],
    ) -> Result<Events<'a, T>, sys::Error<arch::sys_epoll_wait>> {
        let len = self.0.wait(buf, -1)?;
        Ok(Events::new(unsafe { buf.assume_init_mut().get_unchecked_mut(..len) }))
    }
}
