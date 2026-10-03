use core::fmt;

use genos::event::Epoll;
use genos::fd::{AsFd, RawFd};
use seryu::os::Listener;

use crate::event::{EpollBuf, EventKind};
use crate::rt::{FdTable, Queue, Task, Tasks};

mod alloc;

mod client;
mod event;

mod rt;

fn main() -> Result<(), Error> {
    alloc::init_brk();

    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;

    let mut epoll = EpollBuf::new_static()?;
    let mut fds = FdTable::new_static();
    let mut tasks = Tasks::new_static();
    let mut queue = Queue::new_static();

    epoll.add_listener(&listener)?;
    fds.insert(&listener);
    fds.insert(&epoll);

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
                    let fd = fds.insert_read(&client, tasks.peek_id());
                    epoll.add_read(&client, fd as u64)?;
                    tasks.insert(Task::new(client));
                }
                None => {
                    let fd = ev.data as RawFd;
                    let prop = fds.get_mut(fd).unwrap();

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

            if task.poll().is_pending() {
                continue;
            }

            let task = tasks.remove(id).unwrap();
            let socket = task.socket();
            epoll.delete(socket)?;
            fds.remove(socket.as_raw_fd());
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
