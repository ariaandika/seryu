use genos::net::Socket;

use crate::error::Error;
use crate::event::{Event, EventKind, PollHandle};
use crate::http::{ClientTask, Clients};
use crate::queue::Queue;
use crate::task::{Task, TaskId};

// ===== ServerTask =====

pub struct ServerTask {
    poll: PollHandle,
    clients: Clients,
}

impl ServerTask {
    pub fn new(poll: PollHandle) -> Result<ServerTask, Error> {
        Ok(Self { poll, clients: Clients::new()? })
    }

    pub fn create_task(&mut self, socket: Socket) {
        let _ = self.create_task_inner(socket);
    }

    fn create_task_inner(&mut self, socket: Socket) -> Result<(), Error> {
        let event = EventKind::Client(self.clients.peek_id());
        self.poll.add(&socket, event)?;
        self.clients.create(socket)?;
        Ok(())
    }

    pub fn delete_task(&mut self, id: TaskId) {
        let Some(task) = self.clients.get_mut(id) else {
            return;
        };
        let _ = self.poll.delete(&task.socket);
        self.clients.delete(id.to_idx());
    }

    pub fn task_mut(&mut self, id: TaskId) -> Option<&mut ClientTask> {
        self.clients.get_mut(id)
    }
}

// ===== Scheduling =====

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
