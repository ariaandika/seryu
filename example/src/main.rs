use core::fmt;

use genos::event::Epoll;
use genos::fd::{AsFd, RawFd};
use seryu::os::Listener;

use crate::alloc::MemoryPool;
use crate::event::{EpollBuf, EventKind};
use crate::http::Task;
use crate::rt::{FdTable, Queue, Tasks};

mod alloc;
mod event;
mod rt;

mod http;

fn main() -> Result<(), Error> {
    alloc::init_brk();

    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;

    let mut epoll = EpollBuf::new_static()?;
    let mut fds = FdTable::new_static();
    let mut tasks = Tasks::new_static();
    let mut queue = Queue::new_static();
    let mut mpool = MemoryPool::new()?;

    epoll.add_listener(&listener)?;
    fds.insert(&listener)?;
    fds.insert(&epoll)?;

    loop {
        // ===== Event scheduling =====

        use EventKind as Ev;

        while let Some(ev) = epoll.pop_event() {
            match Ev::from_raw(ev.data) {
                Some(Ev::Listener) => {
                    let client = match listener.accept() {
                        Ok(ok) => ok,
                        Err(err) => {
                            if err.is_retry() {
                                continue;
                            } else {
                                return Err(err.into());
                            }
                        }
                    };
                    let fd = fds.insert_read(&client, tasks.peek_id())?;
                    epoll.add_read(&client, fd as u64)?;

                    let mem = mpool.alloc_page()?;
                    tasks.insert(Task::new(client, mem)?);
                }
                None => {
                    let fd = ev.data as RawFd;
                    let prop = fds.get_mut(fd);

                    if ev.events & Epoll::IN == Epoll::IN
                        && let Some(id) = prop.read_task.take()
                        && let Some(task) = tasks.get_mut(id)
                        && !task.is_queued
                    {
                        queue.push(id).unwrap();
                        task.is_queued = true;
                    }

                    if ev.events & Epoll::OUT == Epoll::OUT
                        && let Some(id) = prop.write_task.take()
                        && let Some(task) = tasks.get_mut(id)
                        && !task.is_queued
                    {
                        queue.push(id).unwrap();
                        task.is_queued = true;
                    }
                }
            }
        }

        // ===== Task execution =====

        for &id in queue.as_slice() {
            let task = tasks.get_mut(id).unwrap();
            task.is_queued = false;

            if task.poll().is_pending() {
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

        epoll.wait_buf()?;
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
