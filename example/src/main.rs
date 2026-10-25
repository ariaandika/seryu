#![allow(clippy::module_inception)]
use genos::event::Epoll;
use seryu::os::Listener;

use crate::alloc::MemoryPool;
use crate::error::Error;
use crate::event::{EventBuf, EventKind, EventSource};
use crate::http::HttpTask;
use crate::task::{ContextData, Queue, Task, Tasks, WakerData, WithTask};

mod alloc;
mod error;
mod io;

mod task;

mod event;
mod http;
mod pg;

fn main() -> Result<(), Error> {
    let _ = pg::connect();

    // ===== os =====

    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;
    let event_source = EventSource::new()?;

    event_source.add(&listener, EventKind::Listener)?;

    // ===== data =====

    let mut mpool = MemoryPool::new()?;
    let mut tasks = Tasks::new()?;
    let mut queue = Queue::new()?;
    let mut events = EventBuf::new()?;

    loop {
        // ===== Wait for Events =====

        let events = event_source.wait(&mut *events)?;

        // ===== Event Scheduling =====

        for (ev, data) in events {
            match data {
                EventKind::Listener => {
                    // accept socket
                    let socket = match listener.accept() {
                        Ok(ok) => ok,
                        Err(err) => {
                            if err.is_retry() {
                                continue;
                            } else {
                                return Err(err.into());
                            }
                        }
                    };

                    // register socket to epoll
                    event_source.add(&socket, EventKind::Client(tasks.peek_id()))?;

                    // allocate memory
                    let mem = mpool.alloc_page()?;

                    // create the task
                    tasks.insert(HttpTask::new(socket, mem));
                }
                EventKind::Client(task_id) => {
                    let Some(task) = tasks.get_mut(task_id) else {
                        continue;
                    };

                    let state = task.state();
                    if state.has(Task::QUEUE) {
                        // prevent double queues
                        continue;
                    }

                    let want_rd = state.has(Task::RD) & (ev & Epoll::IN == Epoll::IN);
                    let want_wr = state.has(Task::WR) & (ev & Epoll::OUT == Epoll::OUT);

                    if !(want_rd | want_wr) {
                        // prevent spurious wake up
                        continue;
                    }

                    queue.push(task_id).unwrap();
                    task.set_state(state.unset(Task::RD | Task::WR) | Task::QUEUE);
                }
            }
        }

        // ===== Task Execution =====

        for id in queue.drain() {
            let Some(task) = tasks.get_mut(id) else {
                // dangling task id in scheduler
                continue;
            };
            task.set_state(task.state().unset(Task::QUEUE));

            let waker_data = WakerData::new(id);
            let waker = unsafe { waker_data.create_waker() };
            let mut cx = ContextData::new(&waker);
            let res = task.poll(cx.as_mut_context());

            // io polling will assign interest in task state stored in context
            task.set_state(task.state() | cx.state());

            // TODO: sync write interest with epoll interest

            if res.is_pending() {
                continue;
            }

            // ===== task deallocation =====

            let task = tasks.remove(id).unwrap();
            event_source.delete(&task.socket)?;
            mpool.release(task.page);
        }
    }
}
