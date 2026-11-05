use core::mem::MaybeUninit;

use genos::fd::{AsFd, BorrowedFd};
use genos::io;
use genos::signal::{Signalfd, Signo, Sigset, signalfd};

use crate::error::Error;

pub struct Signal(Signalfd);

impl AsFd for Signal {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Signal {
    pub fn new() -> Result<Self, Error> {
        let mut sigset = Sigset::new();
        sigset.add(Signo::INT);
        sigset.add(Signo::TERM);
        sigset.block()?;
        let fd = Signalfd::create(&sigset, <_>::default())?;
        Ok(Self(fd))
    }

    /// Returns `true` if signal has been received.
    pub fn read(&self) -> Result<Option<signalfd::Siginfo>, Error> {
        let mut info = MaybeUninit::<signalfd::Siginfo>::uninit();
        let buf = unsafe {
            &mut *info
                .as_mut_ptr()
                .cast::<[_; size_of::<signalfd::Siginfo>()]>()
        };
        if io::read(&self.0, buf)? == size_of_val(&info) {
            Ok(Some(unsafe { info.assume_init() }))
        } else {
            Ok(None)
        }
    }
}
