#![allow(clippy::module_inception)]
use seryu::alloc::{Arena, Brk};

use crate::error::Error;
use crate::event::{EventKind, Polling};
use crate::global::{Global, setup_global};
use crate::http::ServerTask;
use crate::os::Listener;
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

    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;
    let polling = Polling::new()?;

    let arena = Arena::<_, { alloc::PAGE_SIZE }>::new(Brk)?;
    let mut events = Polling::event_buf(&arena)?;
    let mut queue = Queue::new(&arena)?;
    let mut server = ServerTask::new()?;

    polling.add(&listener, EventKind::Listener)?;

    setup_global(&Global { poll: &polling });

    loop {
        // ===== Wait for Events =====
        let events = polling.wait(&mut *events)?;

        // ===== Schedule Tasks =====
        for event in events {
            use EventKind as Kind;
            match event.data() {
                Kind::Listener => {
                    for socket in listener.accept() {
                        server.create_task(socket)
                    }
                }
                Kind::Client(id) => {
                    if server.schedule_client(id, event) {
                        queue.push(id).expect("queue full");
                    }
                }
            }
        }

        // ===== Task Execution =====
        for id in queue.drain() {
            use task::{ContextData, Interest, WakerData};

            let Some(task) = server.task_mut(id) else {
                // dangling task id in scheduler
                continue;
            };
            task.interest = task.interest.unset(Interest::QUEUE);

            let waker_data = WakerData::new(id);
            let waker = unsafe { waker_data.create_waker() };
            let mut cx = ContextData::new(&waker);

            // poll the task
            if task.poll(cx.as_mut_context()).is_pending() {
                continue;
            }

            server.delete_task(id);
        }
    }
}
