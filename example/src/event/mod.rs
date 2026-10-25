use core::mem;
use core::ptr::NonNull;

use genos::event::epoll::{self, Epoll};
use genos::fd::AsFd;
use genos::{arch, sys};

use crate::alloc;
use crate::task::TaskId;

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

// ===== EventKind =====

/// Event data.
#[derive(Clone, Copy)]
pub enum EventKind {
    Listener,
    Client(TaskId),
}

impl EventData for EventKind {
    fn from_data(data: u64) -> Self {
        unsafe { mem::transmute(data) }
    }

    fn into_data(self) -> u64 {
        unsafe { mem::transmute(self) }
    }
}

// ===== Events =====

const CAP: usize = alloc::PAGE_SIZE / size_of::<epoll::Event>();

pub struct Events {
    buf: NonNull<[epoll::Event; CAP]>,
    len: usize,
    off: usize,
}

impl Drop for Events {
    fn drop(&mut self) {
        alloc::free_array(self.buf);
    }
}

impl Events {
    pub fn new() -> alloc::Result<Self> {
        alloc::array().map(|buf| Events { buf, len: 0, off: 0 })
    }

    pub fn add<Fd: AsFd, D: EventData>(
        &self,
        fd: &Fd,
        data: D,
        epoll: &Epoll,
    ) -> Result<(), sys::Error<arch::sys_epoll_ctl>> {
        epoll.add(fd, Epoll::IN | Epoll::ET, data.into_data())
    }

    pub fn pop<D: EventData>(&mut self) -> Option<(epoll::EventType, D)> {
        if self.off == self.len {
            return None;
        }
        let ev = unsafe { self.buf.cast::<epoll::Event>().add(self.off).as_ref() };
        self.off += 1;
        Some((ev.events, D::from_data(ev.data)))
    }

    pub fn wait(&mut self, epoll: &Epoll) -> Result<(), sys::Error<arch::sys_epoll_wait>> {
        let buf = unsafe { self.buf.cast::<[_; CAP]>().as_mut() };
        self.len = epoll.wait(buf, -1)?;
        self.off = 0;
        Ok(())
    }
}
