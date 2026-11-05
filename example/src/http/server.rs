use core::pin::Pin;

use genos::net::Socket;

use crate::error::Error;
use crate::event::{Event, EventKind};
use crate::global;
use crate::http::{ClientTask, Clients};
use crate::task::{Interest, TaskId};

// ===== ServerTask =====

pub struct ServerTask {
    clients: Clients,
}

impl ServerTask {
    pub fn new() -> Result<ServerTask, Error> {
        Ok(Self { clients: Clients::new()? })
    }

    pub fn create_task(&mut self, socket: Socket) {
        let _ = self.create_task_inner(socket);
    }

    fn create_task_inner(&mut self, socket: Socket) -> Result<(), Error> {
        let event = EventKind::Client(self.clients.peek_id());
        global::poll().add(&socket, event)?;
        self.clients.create(socket)?;
        Ok(())
    }

    pub fn delete_task(&mut self, id: TaskId) {
        let Some(task) = self.clients.get_mut(id) else {
            return;
        };
        let _ = global::poll().delete(&task.socket);
        self.clients.delete(id.to_idx());
    }

    pub fn task_mut(&mut self, id: TaskId) -> Option<Pin<&mut ClientTask>> {
        self.clients.get_mut(id).map(Pin::new)
    }
}

// ===== Scheduling =====

impl ServerTask {
    /// Returns `true` if given task should be scheduled.
    pub fn schedule_client(&mut self, id: TaskId, event: &Event) -> bool {
        let Some(task) = self.clients.get_mut(id) else {
            return false;
        };

        let state = task.interest;
        let want_rd = state.has(Interest::READ) & event.has_read();
        let want_wr = state.has(Interest::WRITE) & event.has_write();

        // prevent double queues
        if state.has(Interest::QUEUE) {
            return false;
        }

        // prevent not interested event
        if !(want_rd | want_wr) {
            return false;
        }

        task.interest = state.unset(Interest::READ | Interest::WRITE) | Interest::QUEUE;
        true
    }
}
