use core::task::Poll;

use genos::arch;
use genos::sys::{self, ErrCode};
use seryu::os::Listener;

use crate::error::Error;
use crate::event::{Event, EventKind, PollHandle};
use crate::http::Clients;
use crate::queue::Queue;
use crate::task::{ContextData, Task, TaskId, WakerData};

// ===== ServerTask =====

pub struct ServerTask {
    listener: Listener,
    poll: PollHandle,
    clients: Clients,
}

impl ServerTask {
    pub fn new(addr: [u8; 4], port: u16, poll: PollHandle) -> Result<ServerTask, Error> {
        let listener = Listener::bind_tcp(addr, port)?;
        poll.add(&listener, EventKind::Listener)?;

        let clients = Clients::new()?;

        Ok(Self { listener, poll, clients })
    }
}

// ===== Scheduling =====

impl ServerTask {
    pub fn schedule_listener(&mut self) {
        while !matches!(self.listener_inner(), Poll::Pending) {}
    }

    fn listener_inner(&mut self) -> Poll<Result<(), Error>> {
        let socket = match self.listener.accept() {
            Ok(ok) => ok,
            Err(err) => return listener_error(err),
        };

        let event = EventKind::Client(self.clients.peek_id());
        self.poll.add(&socket, event)?;
        self.clients.create(socket)?;

        Poll::Ready(Ok(()))
    }
}

impl ServerTask {
    pub fn schedule_client(&mut self, id: TaskId, event: &Event, queue: &mut Queue<TaskId>) {
        let Some(task) = self.clients.get_mut(id) else {
            return;
        };

        let state = task.task;
        let want_rd = state.has(Task::RD) & event.has_read();
        let want_wr = state.has(Task::WR) & event.has_write();

        // prevent double queues
        if state.has(Task::QUEUE) {
            return;
        }

        // prevent not interested event
        if !(want_rd | want_wr) {
            return;
        }

        if queue.push(id).is_err() {
            // TODO: handle full task queue
            panic!("queue full")
        }

        task.task = state.unset(Task::RD | Task::WR) | Task::QUEUE;
    }
}

// ===== Execution =====

impl ServerTask {
    pub fn execute(&mut self, queue: &mut Queue<TaskId>) {
        for id in queue.drain() {
            let Some(task) = self.clients.get_mut(id) else {
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

            // ===== task deallocation =====

            if let Err(err) = self.poll.delete(&task.socket) {
                eprintln!("{err}");
            }
            self.clients.delete(id.to_idx());
        }
    }
}

// ===== helpers =====

fn listener_error(err: sys::Error<arch::sys_accept4>) -> Poll<Result<(), Error>> {
    // TODO: reschedule accept when out of file descriptor
    match err.code() {
        ErrCode::ENFILE | ErrCode::EMFILE => panic!("out of file descriptor"),
        ErrCode::EAGAIN | ErrCode::EWOULDBLOCK => Poll::Pending,
        _ => Poll::Ready(Err(err.into())),
    }
}
