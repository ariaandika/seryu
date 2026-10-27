use core::task::Poll;

use genos::arch;
use genos::sys::{self, ErrCode};
use seryu::alloc::Slab;
use seryu::os::Listener;

use crate::alloc;
use crate::error::Error;
use crate::event::{Event, EventKind, PollHandle};
use crate::http::{ClientTask, MemoryPool};
use crate::queue::Queue;
use crate::task::{ContextData, Task, TaskId, WakerData};

// ===== ServerTask =====

const CAP: usize = 512;

pub struct ServerTask {
    listener: Listener,
    poll: PollHandle,
    mpool: MemoryPool,
    tasks: Slab<ClientTask>,
}

impl ServerTask {
    pub fn new(addr: [u8; 4], port: u16, poll: PollHandle) -> Result<ServerTask, Error> {
        let listener = Listener::bind_tcp(addr, port)?;
        poll.add(&listener, EventKind::Listener)?;

        let mpool = MemoryPool::new()?;
        let tasks = unsafe {
            Slab::from_buf(
                alloc::array::<ClientTask, CAP>()?.as_ptr().cast(),
                size_of::<[ClientTask; CAP]>(),
            )
        };

        Ok(Self { listener, poll, mpool, tasks })
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

        let data = EventKind::Client(TaskId::from_idx(self.tasks.peek_key()));
        self.poll.add(&socket, data)?;

        let mem = self.mpool.alloc_page()?;
        self.tasks.insert(ClientTask::new(socket, mem))?;
        Poll::Ready(Ok(()))
    }
}

impl ServerTask {
    pub fn schedule_client(&mut self, id: TaskId, event: &Event, queue: &mut Queue<TaskId>) {
        let Some(task) = self.tasks.get_mut(id.to_idx()) else {
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
            let Some(task) = self.tasks.get_mut(id.to_idx()) else {
                // dangling task id in scheduler
                continue;
            };
            task.task = task.task.unset(Task::QUEUE);

            let waker_data = WakerData::new(id);
            let waker = unsafe { waker_data.create_waker() };
            let mut cx = ContextData::new(&waker);

            // poll the task
            let res = task.poll(cx.as_mut_context());

            // TODO: sync write interest with epoll interest

            if res.is_pending() {
                continue;
            }

            // ===== task deallocation =====

            let Some(task) = self.tasks.remove(id.to_idx()) else {
                continue;
            };
            let _ = self.poll.delete(&task.socket);
            self.mpool.release(task.page);
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
