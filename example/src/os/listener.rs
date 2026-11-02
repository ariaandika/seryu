use genos::fd::{AsFd, BorrowedFd};
use genos::net::{SockAddrIn, Socket, opt};
use genos::sys::ErrCode;

use crate::error::Error;

// ===== Listener =====

#[derive(Debug)]
pub struct Listener {
    socket: Socket,
}

impl AsFd for Listener {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.socket.as_fd()
    }
}

impl Listener {
    #[inline]
    pub fn bind_tcp(addr: [u8; 4], port: u16) -> Result<Self, Error> {
        let flags = Socket::CLOEXEC | Socket::NONBLOCK;
        let socket = Socket::create(Socket::INET, Socket::STREAM, flags)?;

        let value = opt::from_bool(true);
        socket.setopt(opt::REUSEADDR, opt::SOCKET, value.as_value(), value.size())?;

        let addr = u32::from_ne_bytes(addr);
        let ip_addr = SockAddrIn::new(port.swap_bytes(), addr);
        socket.bind(ip_addr.as_sockaddr(), ip_addr.addrlen())?;

        socket.listen(-1)?;
        Ok(Self { socket })
    }

    pub fn accept(&self) -> Accept<'_> {
        Accept { listener: self }
    }
}

// ===== Accept =====

pub struct Accept<'a> {
    listener: &'a Listener,
}

impl Iterator for Accept<'_> {
    type Item = Socket;

    fn next(&mut self) -> Option<Self::Item> {
        match self
            .listener
            .socket
            .accept(None, None, Socket::CLOEXEC | Socket::NONBLOCK)
        {
            Ok(socket) => Some(socket),
            Err(err) => {
                use ErrCode as E;
                match err.code() {
                    E::ENFILE | E::EMFILE => panic!("out of file descriptor"),
                    E::EAGAIN | E::EWOULDBLOCK => None,
                    _ => {
                        eprintln!("{err}");
                        None
                    }
                }
            }
        }
    }
}
