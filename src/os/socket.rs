use core::ffi::CStr;
use core::mem::MaybeUninit;
use core::task::Poll;
use core::{error, fmt};

use genos::fd::{AsFd, BorrowedFd};
use genos::net::{SockAddrIn, Socket as socket, opt};
use genos::{arch, io, net, sys};

use crate::os::{ReadError, WriteError};

#[derive(Debug)]
#[repr(transparent)]
pub struct Socket(socket);

impl AsFd for Socket {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Socket {
    fn create(sock: net::Family) -> Result<socket, sys::Error<arch::sys_socket>> {
        socket::create(sock, socket::STREAM, socket::CLOEXEC | socket::NONBLOCK)
    }

    fn connect(
        sock: net::Family,
        addr: &net::SockAddr,
        addrlen: net::Socklen,
    ) -> Result<Self, SocketError> {
        let socket = Self::create(sock)?;
        socket.connect(addr, addrlen)?;
        Ok(Self(socket))
    }

    #[inline]
    pub fn connect_unix(path: &CStr) -> Result<Self, SocketError> {
        let addr = net::SockAddrUn::from_path(path.into());
        Self::connect(socket::UNIX, addr.as_sockaddr(), addr.addrlen())
    }

    #[inline]
    pub fn connect_tcp(addr: [u8; 4], port: u16) -> Result<Self, SocketError> {
        let addr = net::SockAddrIn::new(port.swap_bytes(), u32::from_ne_bytes(addr));
        Self::connect(socket::INET, addr.as_sockaddr(), addr.addrlen())
    }

    #[inline]
    pub fn bind_tcp(addr: [u8; 4], port: u16) -> Result<Socket, SocketError> {
        let socket = Self::create(socket::INET)?;
        let opt = opt::from_bool(true);
        socket.setopt(opt::REUSEADDR, opt::SOCKET, opt.as_value(), opt.size())?;

        let ip_addr = SockAddrIn::new(port.swap_bytes(), u32::from_ne_bytes(addr));
        socket.bind(ip_addr.as_sockaddr(), ip_addr.addrlen())?;
        socket.listen(-1)?;
        Ok(Self(socket))
    }
}

impl Socket {
    #[inline]
    pub fn poll_read(&self, buf: &mut [MaybeUninit<u8>]) -> Poll<Result<usize, ReadError>> {
        self.would_block(|s| io::read(s, buf))
    }

    #[inline]
    pub fn poll_write(&self, buf: &[u8]) -> Poll<Result<usize, WriteError>> {
        self.would_block(|s| io::write(s, buf))
    }

    #[inline]
    pub fn poll_accept(&self) -> Poll<Result<Socket, sys::Error<arch::sys_accept4>>> {
        self.would_block(|s| {
            s.accept(None, None, socket::CLOEXEC | socket::NONBLOCK)
                .map(Self)
        })
    }

    fn would_block<T, E, F: FnOnce(&socket) -> Result<T, sys::Error<E>>>(
        &self,
        f: F,
    ) -> Poll<Result<T, sys::Error<E>>> {
        match f(&self.0) {
            Ok(ok) => Poll::Ready(Ok(ok)),
            Err(err) => match err.code() {
                sys::ErrCode::EAGAIN | sys::ErrCode::EWOULDBLOCK => Poll::Pending,
                _ => Poll::Ready(Err(err)),
            },
        }
    }
}

// ===== Error =====

#[derive(Debug)]
pub enum SocketError {
    Create(sys::Error<arch::sys_socket>),
    Connect(sys::Error<arch::sys_connect>),
    Opt(sys::Error<arch::sys_setsockopt>),
    Bind(sys::Error<arch::sys_bind>),
    Listen(sys::Error<arch::sys_listen>),
}

impl error::Error for SocketError {}

macro_rules! impl_from {
    ($($v:ident(sys::Error<arch::$err:ident>),)*) => {
        $(
            impl From<sys::Error<arch::$err>> for SocketError {
                fn from(v: sys::Error<arch::$err>) -> Self {
                    Self::$v(v)
                }
            }
        )*
        impl fmt::Display for SocketError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(Self::$v(e) => e.fmt(f)),*
                }
            }
        }
    };
}

impl_from! {
    Create(sys::Error<arch::sys_socket>),
    Connect(sys::Error<arch::sys_connect>),
    Opt(sys::Error<arch::sys_setsockopt>),
    Bind(sys::Error<arch::sys_bind>),
    Listen(sys::Error<arch::sys_listen>),
}
