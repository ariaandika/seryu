use core::task::Poll;

use genos::net::Socket;
use seryu::alloc::Slab;

use crate::alloc;
use crate::client::HttpState;

#[derive(Debug, Clone, Copy)]
pub struct TaskId(u32);

pub struct Task {
    socket: Socket,
    service: HttpState,
    pub is_queued: bool,
}

impl Task {
    pub fn new(socket: Socket) -> Self {
        Self { socket, service: HttpState::new(), is_queued: false }
    }

    pub fn socket(&self) -> &Socket {
        &self.socket
    }

    pub fn poll(&mut self) -> Poll<()> {
        self.service.on_socket(&self.socket)
    }
}

// ===== Tasks =====

const CAP: usize = 512;

pub struct Tasks {
    buf: Slab<Task>,
}

impl Tasks {
    pub fn new_static() -> Self {
        let ptr = alloc::leak_slice::<Task>(512).as_ptr().cast();
        let buf = unsafe { Slab::from_buf(ptr, CAP * size_of::<Task>()) };
        Self { buf }
    }

    pub fn peek_id(&self) -> TaskId {
        TaskId(self.buf.peek_key().try_into().unwrap())
    }

    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.buf.get_mut(id.0 as usize)
    }

    pub fn insert(&mut self, task: Task) -> TaskId {
        self.buf
            .insert(task)
            .map(|id| TaskId(id.try_into().unwrap()))
            .unwrap()
    }

    pub fn remove(&mut self, id: TaskId) -> Option<Task> {
        self.buf.remove(id.0 as usize)
    }
}
