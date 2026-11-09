#![allow(clippy::module_inception)]
use core::pin;

use seryu::alloc::{Arena, Brk};

use crate::app::{EventBuf, EventKind, GlobalRt, Rt};
use crate::error::Error;
use crate::http::{Clients, Request, Server};
use crate::os::Signal;

// OS
mod alloc;
mod error;
mod os;

// shared
mod shared;

// protocol
mod http;
mod pg;

// application
mod app;

fn main() -> Result<(), Error> {
    let _ = pg::connect();

    let arena = Arena::<_, { alloc::PAGE_SIZE }>::new(Brk)?;
    let mut events = EventBuf::uninit_array(&arena)?;

    let pin = pin::pin!(Rt::new()?);
    pin.setup();

    let signal = Signal::new()?;
    let mut server = Server::new([127, 0, 0, 1], 4040)?;
    let mut clients = Clients::<GlobalRt>::new()?;

    GlobalRt.poll.add(&signal, EventKind::Signal)?;
    GlobalRt.poll.add(&server, EventKind::Listener)?;

    loop {
        // ===== Wait for Events =====
        let events = GlobalRt.poll.wait(&mut *events)?;

        // ===== Schedule Tasks =====
        for event in events {
            use EventKind as Kind;
            match event.data() {
                Kind::Listener => server.poll(&mut clients),
                Kind::Client(id) => clients.poll(id, event),
                Kind::Signal => {
                    if signal.read()?.is_some() {
                        return Ok(());
                    }
                }
            }
        }
    }
}

async fn request_handler(request: Request) {
    println!(
        "{} {}",
        String::from_utf8_lossy(request.method()),
        String::from_utf8_lossy(request.target())
    );
    for header in request.headers() {
        println!("{header:?}");
    }
}
