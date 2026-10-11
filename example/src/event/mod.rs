use core::ptr::NonNull;
use core::slice;

use genos::event::epoll::{self, Epoll};
use genos::fd::AsFd;
use genos::{arch, sys};

use crate::alloc;

// ===== Event =====

/// Mark special event.
const EV_FLAG: u64 = !(u64::MAX >> 1);

/// A special os event.
///
/// This event may have special treatment than arbitrary os event.
pub enum Event {
    Listener,
}

impl Event {
    pub fn from_epoll(event: &epoll::Event) -> Option<Self> {
        if event.data & EV_FLAG == EV_FLAG { Some(Self::Listener) } else { None }
    }

    pub fn add_with<Fd: AsFd>(
        epoll: &Epoll,
        listener: &Fd,
        event: Self,
    ) -> Result<(), sys::Error<arch::sys_epoll_ctl>> {
        epoll.add(listener, Epoll::IN | Epoll::ET, event as u64 | EV_FLAG)
    }
}

// ===== Events =====

const CAP: usize = 255;

pub struct Events {
    buf: NonNull<epoll::Event>,
    len: usize,
    off: usize,
}

impl Events {
    pub fn new_static() -> Self {
        let buf = alloc::leak_slice(CAP);
        Events { buf, len: 0, off: 0 }
    }

    pub fn pop(&mut self) -> Option<&epoll::Event> {
        if self.off == self.len {
            return None;
        }
        let ptr = unsafe { self.buf.add(self.off) };
        self.off += 1;
        Some(unsafe { ptr.cast::<epoll::Event>().as_ref() })
    }

    pub fn wait(&mut self, epoll: &Epoll) -> Result<(), sys::Error<arch::sys_epoll_wait>> {
        let buf = unsafe { slice::from_raw_parts_mut(self.buf.as_ptr().cast(), CAP) };
        self.len = epoll.wait(buf, -1)?;
        self.off = 0;
        Ok(())
    }
}
