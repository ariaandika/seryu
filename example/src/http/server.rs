use core::task::Poll;

use genos::fd::{AsFd, BorrowedFd};
use seryu::os::Socket;

use crate::error::Error;

// ===== ServerTask =====

pub struct Server {
    listener: Socket,
}

impl AsFd for Server {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.listener.as_fd()
    }
}

impl Server {
    pub fn new(addr: [u8; 4], port: u16) -> Result<Self, Error> {
        Ok(Self { listener: Socket::bind_tcp(addr, port)? })
    }
}

// ===== Server Listener =====

pub trait Spawner {
    fn spawn(&mut self, socket: Socket);
}

impl Server {
    pub fn poll<S: Spawner>(&mut self, mut spawner: S) {
        loop {
            match self.listener.poll_accept() {
                Poll::Ready(Ok(socket)) => spawner.spawn(socket),
                Poll::Ready(Err(err)) => {
                    eprintln!("{err}");
                    break;
                }
                Poll::Pending => break,
            }
        }
    }
}
