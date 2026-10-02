use core::fmt;

use seryu::os::Listener;

use crate::event::{EpollBuf, EventKind};
use crate::fd_table::FdTable;

mod alloc;
mod fd_table;

mod client;
mod event;

fn main() -> Result<(), Error> {
    alloc::init_brk();

    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;

    let mut epoll = EpollBuf::new_static()?;
    epoll.add_listener(&listener)?;

    let mut fds = FdTable::new_static();
    fds.insert(&listener);
    fds.insert(&epoll);

    loop {
        let Some(ev) = epoll.pop_event() else {
            epoll.wait_buf()?;
            continue;
        };

        use EventKind as Ev;
        match Ev::from_event(ev) {
            Some(Ev::Listener) => {
                let client = listener.accept()?;
                fds.insert(&client);
                client::handle(&client)?;
                fds.remove(&client);
            }
            None => {
                // client
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
