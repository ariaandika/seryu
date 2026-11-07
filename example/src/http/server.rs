use core::task::Poll;

use seryu::os::Socket;

use crate::error::Error;
use crate::event::EventKind;
use crate::http::Clients;
use crate::rt::Reactor;

// ===== ServerTask =====

pub struct Server<R: Reactor> {
    reactor: R,
    listener: Socket,
}

impl<R: Reactor> Drop for Server<R> {
    fn drop(&mut self) {
        let _ = self.reactor.delete(&self.listener);
    }
}

impl<R: Reactor> Server<R> {
    pub fn new(addr: [u8; 4], port: u16, reactor: R) -> Result<Self, Error> {
        let listener = Socket::bind_tcp(addr, port)?;
        reactor.add(&listener, EventKind::Listener)?;
        Ok(Self { reactor, listener })
    }
}

// ===== Server Listener =====

impl<R: Reactor> Server<R> {
    pub fn poll<R2: Reactor>(&mut self, clients: &mut Clients<R2>) {
        loop {
            match self.listener.poll_accept() {
                Poll::Ready(Ok(socket)) => {
                    if clients.create(socket).is_err() {
                        todo!("failed to create client task")
                    }
                }
                Poll::Ready(Err(err)) => {
                    eprintln!("{err}");
                    break;
                }
                Poll::Pending => break,
            }
        }
    }
}
