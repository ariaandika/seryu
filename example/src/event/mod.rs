use core::ptr::NonNull;
use core::{ops, slice};

use genos::event::Epoll;
use genos::event::epoll::{self, Event};
use genos::fd::{AsFd, BorrowedFd};
use seryu::error::IoError;

use crate::alloc;

// ===== EpollKind =====

const EV_FLAG: u64 = !(u64::MAX >> 1);
const LISTENER: u64 = 1;

pub enum EventKind {
    Listener,
}

impl EventKind {
    pub fn from_raw(raw: u64) -> Option<Self> {
        if raw & EV_FLAG == EV_FLAG { Some(Self::Listener) } else { None }
    }
}

// ===== EpollBuf =====

const CAP: usize = 255;

pub struct EpollBuf {
    buf: NonNull<Event>,
    len: usize,
    off: usize,
    fd: epoll::Epoll,
}

impl AsFd for EpollBuf {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl EpollBuf {
    pub fn new_static() -> Result<EpollBuf, IoError> {
        let buf = alloc::leak_slice(CAP);
        let fd = Epoll::create(Epoll::CLOEXEC)?;
        Ok(EpollBuf { buf, len: 0, off: 0, fd })
    }

    pub fn add_listener<Fd: AsFd>(&self, socket: &Fd) -> Result<(), IoError> {
        self.add(socket, Epoll::IN | Epoll::ET, LISTENER | EV_FLAG)
            .map_err(<_>::into)
    }

    pub fn add_read<Fd: AsFd>(&self, fd: &Fd, key: u64) -> Result<(), IoError> {
        self.add(fd, Epoll::IN | Epoll::ET, key).map_err(<_>::into)
    }

    pub fn pop_event(&mut self) -> Option<Event> {
        if self.off != self.len {
            let ptr = unsafe { self.buf.add(self.off) };
            self.off += 1;
            Some(unsafe { ptr.cast::<Event>().read() })
        } else {
            None
        }
    }

    pub fn wait_buf(&mut self) -> Result<(), IoError> {
        let buf = unsafe { slice::from_raw_parts_mut(self.buf.as_ptr().cast(), CAP) };
        self.len = self.fd.wait(buf, -1)?;
        self.off = 0;
        Ok(())
    }
}

impl ops::Deref for EpollBuf {
    type Target = epoll::Epoll;

    fn deref(&self) -> &Self::Target {
        &self.fd
    }
}
