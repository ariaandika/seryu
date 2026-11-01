#![allow(clippy::module_inception)]
use crate::error::Error;
use crate::event::{EventKind, Polling};
use crate::queue::Queue;

// OS Abstraction
mod alloc;
mod error;

// Global Abstraction
mod task;

// Global State
mod event;
mod queue;

// Application Domain
mod http;
mod pg;

fn main() -> Result<(), Error> {
    let _ = pg::connect();

    // ===== os =====

    let polling = Polling::new()?;

    let mut server = http::ServerTask::new([127, 0, 0, 1], 4040, polling.handle())?;

    // ===== data =====

    let mut queue = Queue::new()?;
    let mut events = Polling::event_buf()?;

    loop {
        // ===== Wait for Events =====
        let events = polling.wait(&mut *events)?;

        // ===== Schedule Tasks =====
        for event in events {
            use EventKind as Kind;
            match event.data() {
                Kind::Listener => server.schedule_listener(),
                Kind::Client(id) => server.schedule_client(id, event, &mut queue),
            }
        }

        // ===== Task Execution =====
        server.execute(&mut queue);
    }
}
