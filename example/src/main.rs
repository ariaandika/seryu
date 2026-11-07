#![allow(clippy::module_inception)]
use core::task::Context;

use seryu::alloc::{Arena, Brk};

use crate::error::Error;
use crate::event::{EpollHandle, EventBuf, EventKind};
use crate::http::{Clients, Request, Server};
use crate::os::Signal;
use crate::queue::Queue;

// OS Abstraction
mod alloc;
mod error;
mod os;

// Shared Abstraction
mod rt;
mod task;

// Global State
mod event;
mod queue;

// Application Domain
mod http;
mod pg;

fn main() -> Result<(), Error> {
    let _ = pg::connect();

    let poll = EpollHandle::new_static()?;
    let signal = Signal::new()?;

    let arena = Arena::<_, { alloc::PAGE_SIZE }>::new(Brk)?;
    let mut events = EventBuf::uninit_array(&arena)?;
    let mut queue = Queue::new(&arena)?;

    let mut server = Server::new([127, 0, 0, 1], 4040, poll.clone())?;
    let mut clients = Clients::new(poll.clone())?;

    poll.add(&signal, EventKind::Signal)?;

    loop {
        // ===== Wait for Events =====
        let events = poll.wait(&mut *events)?;

        // ===== Schedule Tasks =====
        for event in events {
            use EventKind as Kind;
            match event.data() {
                Kind::Listener => server.poll(&mut clients),
                Kind::Client(id) => {
                    if clients.schedule_client(id, event) {
                        queue.push(id).expect("queue full");
                    }
                }
                Kind::Signal => {
                    if signal.read()?.is_some() {
                        return Ok(());
                    }
                }
            }
        }

        // ===== Task Execution =====
        for id in queue.drain() {
            let Some(task) = clients.get_mut(id) else {
                // dangling task id in scheduler
                continue;
            };

            let waker = task::waker(&*task);
            let mut cx = Context::from_waker(&waker);

            // poll the task
            if task.poll(&mut cx).is_pending() {
                continue;
            }

            clients.delete(id);
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
