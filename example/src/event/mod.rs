use core::mem::MaybeUninit;
use core::ops;

use genos::event::Epoll;
use genos::event::epoll::{self, Event};
use genos::fd::AsFd;
use seryu::error::IoError;

// ===== EpollKind =====

const EV_FLAG: u64 = !(u64::MAX >> 1);
const LISTENER: u64 = 1;

pub enum EventKind {
    Listener,
}

impl EventKind {
    pub fn from_event(value: Event) -> Option<Self> {
        if value.data & EV_FLAG == EV_FLAG { Some(Self::Listener) } else { None }
    }
}

// ===== EpollBuf =====

pub struct EpollBuf<'a> {
    buf: &'a mut [MaybeUninit<Event>],
    len: usize,
    off: usize,
    fd: epoll::Epoll,
}

impl EpollBuf<'_> {
    pub fn new<'a>(buf: &'a mut [MaybeUninit<Event>]) -> Result<EpollBuf<'a>, IoError> {
        let fd = Epoll::create(Epoll::CLOEXEC)?;
        Ok(EpollBuf { buf, len: 0, off: 0, fd })
    }

    pub fn add_listener<Fd: AsFd>(&self, socket: &Fd) -> Result<(), IoError> {
        self.add(socket, Epoll::IN | Epoll::ET, LISTENER | EV_FLAG)
            .map_err(<_>::into)
    }

    pub fn pop_event(&mut self) -> Option<Event> {
        if self.off != self.len {
            let ptr = unsafe { self.buf.as_mut_ptr().add(self.off) };
            self.off += 1;
            Some(unsafe { ptr.cast::<Event>().read() })
        } else {
            None
        }
    }

    pub fn wait_buf(&mut self) -> Result<(), IoError> {
        self.len = self.fd.wait(self.buf, -1)?;
        self.off = 0;
        Ok(())
    }
}

impl ops::Deref for EpollBuf<'_> {
    type Target = epoll::Epoll;

    fn deref(&self) -> &Self::Target {
        &self.fd
    }
}
