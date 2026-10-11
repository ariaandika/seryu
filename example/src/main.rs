#![allow(clippy::module_inception)]
use core::fmt;
use core::task::Context;

use genos::event::Epoll;
use genos::fd::{AsFd, RawFd};
use seryu::os::Listener;

use crate::alloc::MemoryPool;
use crate::event::{Event, Events};
use crate::http::HttpTask;
use crate::rt::Shared;
use crate::task::{FdTable, Queue, Tasks, WakerData};

mod alloc;
mod io;
mod rt;

mod event;
mod task;

mod http;

fn main() -> Result<(), Error> {
    alloc::init_brk();

    // ===== os =====

    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;
    let epoll = Epoll::create(Epoll::CLOEXEC)?;

    Event::add_with(&epoll, &listener, Event::Listener)?;

    // ===== data =====

    let mut mpool = MemoryPool::new()?;
    let mut tasks = Tasks::new_static();
    let mut queue = Queue::new_static();
    let mut events = Events::new_static();

    let mut shared = Shared { fds: FdTable::new_static() };

    shared.fds.insert(&listener)?;
    shared.fds.insert(&epoll)?;
    shared.setup();

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
                    let fd = shared.fds.insert(&socket)?;
                    WakerData::new(tasks.peek_id()).register_read(&socket);
                    epoll.add(&socket, Epoll::IN | Epoll::ET, fd as u64)?;

                    let mem = mpool.alloc_page()?;
                    let task = HttpTask::new(socket, mem)?;
                    tasks.insert(task);
                }
                None => {
                    let fd = ev.data as RawFd;
                    let prop = shared.fds.get_mut(fd);

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
            let mut cx = Context::from_waker(&waker);
            if task.poll(&mut cx).is_pending() {
                continue;
            }

            // ===== task deallocation =====

            let task = tasks.remove(id).unwrap();
            epoll.delete(&task.socket)?;
            shared.fds.remove(task.socket.as_raw_fd());

            mpool.release(task.page);
        }

        queue.clear();

        // ===== Wait for events =====

        events.wait(&epoll)?;
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
