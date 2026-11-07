#![allow(clippy::module_inception)]
use core::task::{Context, Poll};

use seryu::alloc::{Arena, Brk};
use seryu::os::Socket;

use crate::error::Error;
use crate::event::{EventKind, Polling};
use crate::global::{Global, setup_global};
use crate::http::{Request, ServerTask};
use crate::os::Signal;
use crate::queue::Queue;

// OS Abstraction
mod alloc;
mod error;
mod os;

// Shared Abstraction
mod global;
mod task;

// Global State
mod event;
mod queue;

// Application Domain
mod http;
mod pg;

fn main() -> Result<(), Error> {
    let _ = pg::connect();

    let listener = Socket::bind_tcp([127, 0, 0, 1], 4040)?;
    let polling = Polling::new()?;
    let signal = Signal::new()?;

    let arena = Arena::<_, { alloc::PAGE_SIZE }>::new(Brk)?;
    let mut events = Polling::event_buf(&arena)?;
    let mut queue = Queue::new(&arena)?;
    let mut server = ServerTask::new()?;

    polling.add(&listener, EventKind::Listener)?;
    polling.add(&signal, EventKind::Signal)?;

    setup_global(&Global { poll: &polling });

    loop {
        // ===== Wait for Events =====
        let events = polling.wait(&mut *events)?;

        // ===== Schedule Tasks =====
        for event in events {
            use EventKind as Kind;
            match event.data() {
                Kind::Listener => {
                    while let Poll::Ready(socket) = listener.poll_accept()? {
                        server.create_task(socket)
                    }
                }
                Kind::Client(id) => {
                    if server.schedule_client(id, event) {
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
            let Some(task) = server.task_mut(id) else {
                // dangling task id in scheduler
                continue;
            };

            let waker = task::waker(&*task);
            let mut cx = Context::from_waker(&waker);

            // poll the task
            if task.poll(&mut cx).is_pending() {
                continue;
            }

            server.delete_task(id);
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
