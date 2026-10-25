#![allow(clippy::module_inception)]
use genos::event::Epoll;
use genos::fd::AsFd;
use seryu::os::Listener;

use crate::alloc::MemoryPool;
use crate::error::Error;
use crate::event::{EventKind, Events};
use crate::http::HttpTask;
use crate::task::{ContextData, FdTable, Queue, Tasks, WakerData};

mod alloc;
mod error;
mod io;

mod task;

mod event;
mod http;
mod pg;

fn main() -> Result<(), Error> {
    // ===== os =====

    let _ = pg::connect();
    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;
    let epoll = Epoll::create(Epoll::CLOEXEC)?;

    // ===== data =====

    let mut mpool = MemoryPool::new()?;
    let mut tasks = Tasks::new()?;
    let mut queue = Queue::new()?;
    let mut events = Events::new()?;
    let mut fds = FdTable::new()?;

    fds.insert(&listener);
    fds.insert(&epoll);
    events.add(&listener, EventKind::Listener, &epoll)?;

    loop {
        // ===== Event scheduling =====
        while let Some((ev, data)) = events.pop() {
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
                    let data = EventKind::Client(socket.as_fd());
                    events.add(&socket, data, &epoll).unwrap();

                    // register interest for the new task to the socket
                    let fd_prop = fds.insert(&socket);
                    fd_prop.read = Some(WakerData::new(tasks.peek_id()));

                    // create the task
                    let mem = mpool.alloc_page()?;
                    tasks.insert(HttpTask::new(socket, mem));
                }
                EventKind::Client(fd) => {
                    let prop = fds.get_mut(fd.as_raw_fd());

                    if ev & Epoll::IN == Epoll::IN
                        && let Some(waker) = prop.read.take()
                    {
                        queue.push(waker.id()).unwrap();
                    }

                    if ev & Epoll::OUT == Epoll::OUT
                        && let Some(waker) = prop.write.take()
                    {
                        queue.push(waker.id()).unwrap();
                    }
                }
            }
        }

        // ===== Task execution =====

        for id in queue.drain() {
            let Some(task) = tasks.get_mut(id) else {
                // dangling task id in scheduler
                continue;
            };

            // make sure to not borrow `shared` before `poll()`
            let waker_data = WakerData::new(id);
            // SAFETY: `waker_data` does not move
            let waker = unsafe { waker_data.create_waker() };
            let mut cx = ContextData::new(&waker, &mut fds);
            if task.poll(cx.as_mut_context()).is_pending() {
                continue;
            }

            // ===== task deallocation =====

            let task = tasks.remove(id).unwrap();
            epoll.delete(&task.socket)?;
            fds.remove(task.socket.as_raw_fd());

            mpool.release(task.page);
        }

        // ===== Wait for events =====

        events.wait(&epoll)?;
    }
}
