#![allow(clippy::module_inception)]
use genos::event::Epoll;
use genos::fd::{AsFd, RawFd};
use seryu::os::Listener;

use crate::alloc::MemoryPool;
use crate::error::Error;
use crate::event::{Event, Events};
use crate::http::HttpTask;
use crate::task::{ContextData, FdTable, Queue, Tasks, WakerData};

mod alloc;
mod error;
mod io;

mod event;
mod task;

mod http;
mod pg;

fn main() -> Result<(), Error> {
    alloc::init_brk();

    // ===== os =====

    let _ = pg::connect();
    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;
    let epoll = Epoll::create(Epoll::CLOEXEC)?;
    let mut fds = FdTable::new_static();

    Event::add_with(&epoll, &listener, Event::Listener)?;
    fds.insert(&listener)?;
    fds.insert(&epoll)?;

    // ===== data =====

    let mut mpool = MemoryPool::new()?;
    let mut tasks = Tasks::new_static();
    let mut queue = Queue::new_static();
    let mut events = Events::new_static();

    loop {
        // ===== Event scheduling =====
        while let Some(ev) = events.pop() {
            match Event::from_epoll(ev) {
                Some(Event::Listener) => {
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

                    // register fd to epoll
                    let fd = socket.as_raw_fd();
                    epoll.add(&socket, Epoll::IN | Epoll::ET, fd as u64)?;

                    // register interest for the new task to the fd
                    let fd_prop = fds.insert(&socket)?;
                    fd_prop.read = Some(WakerData::new(tasks.peek_id()));

                    // store the task
                    let mem = mpool.alloc_page()?;
                    let task = HttpTask::new(socket, mem)?;
                    tasks.insert(task);
                }
                None => {
                    let fd = ev.data as RawFd;
                    let prop = fds.get_mut(fd);

                    if ev.events & Epoll::IN == Epoll::IN
                        && let Some(waker) = prop.read.take()
                    {
                        queue.push(waker.id()).unwrap();
                    }

                    if ev.events & Epoll::OUT == Epoll::OUT
                        && let Some(waker) = prop.write.take()
                    {
                        queue.push(waker.id()).unwrap();
                    }
                }
            }
        }

        // ===== Task execution =====

        for &id in queue.as_slice() {
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

        queue.clear();

        // ===== Wait for events =====

        events.wait(&epoll)?;
    }
}
