#![allow(clippy::module_inception)]
use crate::error::Error;
use crate::event::{EventKind, Polling};
use crate::http::ServerTask;
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

    // event polling
    let polling = Polling::new()?;

    // event buffer
    let mut events = Polling::event_buf()?;

    // scheduled task execution
    let mut queue = Queue::new()?;

    // server state manager
    let mut server = ServerTask::new([127, 0, 0, 1], 4040, polling.handle())?;

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
        for id in queue.drain() {
            use task::{ContextData, Task, WakerData};

            let Some(task) = server.task_mut(id) else {
                // dangling task id in scheduler
                continue;
            };
            task.task = task.task.unset(Task::QUEUE);

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
