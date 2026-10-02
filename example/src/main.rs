use core::fmt;
use std::mem::MaybeUninit;

use seryu::os::Listener;

use crate::event::{EpollBuf, EventKind};

mod client;
mod event;

fn main() -> Result<(), Error> {
    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;

    let mut buf = [const { MaybeUninit::uninit() }; 255];
    let mut epoll = EpollBuf::new(&mut buf)?;
    epoll.add_listener(&listener)?;

    loop {
        epoll.wait_buf()?;

        while let Some(ev) = epoll.pop_event() {
            use EventKind as Ev;
            match Ev::from_event(ev) {
                Some(Ev::Listener) => {
                    let client = listener.accept()?;
                    client::handle(client)?;
                }
                None => {
                    // client
                }
            }
        }
    }
}

// ===== Error =====

#[derive(Debug)]
struct Error;

impl<E: fmt::Display> From<E> for Error {
    fn from(value: E) -> Self {
        eprintln!("{value}");
        Self
    }
}
