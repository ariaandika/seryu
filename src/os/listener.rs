use genos::fd::{AsFd, BorrowedFd};
use genos::net::Socket;
use genos::net::addr::Family;
use genos::net::ip::SockAddrIn;
use genos::net::option::{OptName, OptValue};

use crate::error::IoError;

#[derive(Debug)]
pub struct Listener {
    socket: Socket,
}

impl AsFd for Listener {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.socket.as_fd()
    }
}

impl Listener {
    #[inline]
    pub fn bind_tcp(addr: [u8; 4], port: u16) -> Result<Self, IoError> {
        let flags = Socket::CLOEXEC | Socket::NONBLOCK;
        let socket = Socket::create(Family::INET, Socket::STREAM, flags)?;

        let value = OptValue::from_bool(true);
        socket.setopt(OptName::REUSEADDR, value.as_value(), value.size())?;

        let addr = u32::from_ne_bytes(addr);
        let ip_addr = SockAddrIn::new(port.swap_bytes(), addr);
        socket.bind(ip_addr.as_sockaddr(), ip_addr.addrlen())?;

        socket.listen(-1)?;
        Ok(Self { socket })
    }

    #[inline]
    pub fn accept(&self) -> Result<Socket, IoError> {
        self.socket
            .accept(None, None, Socket::CLOEXEC | Socket::NONBLOCK)
            .map_err(<_>::into)
    }
}
