use core::ffi::CStr;
use core::task::{self, Poll, ready};
use core::{fmt, mem};

use genos::net::{self, Socket};
use seryu::alloc::OutOfCapacity;
use seryu::error::{ReadError, WriteError};
use seryu::postgres::backend::{BackendType as B, Message};
use seryu::postgres::{Decode, Encode, backend, frontend};

use crate::alloc;
use crate::error::Error;
use crate::pg::read::ReadBuf;
use crate::pg::write::WriteBuf;

pub struct Postgres {
    socket: Socket,
    read_buf: ReadBuf,
    write_buf: WriteBuf,
}

// ===== Constructor =====

impl Postgres {
    #[allow(unused)]
    pub fn connect_unix(path: &CStr) -> Result<Self, Error> {
        let addr = net::SockAddrUn::from_path(path.into());
        Self::connect(Socket::UNIX, addr.as_sockaddr(), addr.addrlen())
    }

    #[allow(unused)]
    pub fn connect_tcp(addr: [u8; 4], port: u16) -> Result<Self, Error> {
        let addr = net::SockAddrIn::new(port.swap_bytes(), u32::from_ne_bytes(addr));
        Self::connect(Socket::INET, addr.as_sockaddr(), addr.addrlen())
    }

    fn connect(
        sock: net::Family,
        addr: &net::SockAddr,
        addrlen: net::Socklen,
    ) -> Result<Self, Error> {
        // create socket
        let socket = Socket::create(sock, Socket::STREAM, <_>::default())?;
        socket.connect(addr, addrlen)?;

        // allocate message buffer
        let ptr = alloc::leak_slice(ReadBuf::CAPACITY + WriteBuf::CAPACITY);
        let read_buf = unsafe { ReadBuf::from_raw(ptr) };
        let write_buf = unsafe { WriteBuf::from_raw(ptr.add(ReadBuf::CAPACITY)) };
        Ok(Self { socket, read_buf, write_buf })
    }
}

// ===== Read =====

impl Postgres {
    fn read_sync(&mut self) -> Result<bool, ReadError> {
        self.read_buf.read_fd_sync(&self.socket)
    }

    /// Read one message.
    ///
    /// This converts `ErrorResponse` to `Result::Err`, and consume `NoticeResponse` and
    /// `ParameterStatus`.
    pub fn poll_msg<'a>(&'a mut self, cx: &mut task::Context) -> Poll<Result<Message<'a>, Error>> {
        loop {
            let Some(msg) = self.read_buf.read_msg()? else {
                if ready!(self.read_buf.poll_read_fd(&self.socket, cx))? {
                    return Poll::Ready(Err("connection terminated".into()));
                }
                continue;
            };
            match msg.ty() {
                B::ErrorResponse => return Poll::Ready(as_error(msg)),
                B::NoticeResponse => {
                    println!("notice: {}", msg.decode::<backend::NoticeResponse>().unwrap())
                }
                B::ParameterStatus => {} // TODO: pg: parameter status
                _ => {
                    // SAFETY: impossible: https://users.rust-lang.org/t/mutably-borrowed-in-previous-iteration-despite-no-chance-of-conflicting-borrow/83274/3
                    let msg = unsafe { mem::transmute::<Message<'_>, Message<'_>>(msg) };
                    return Poll::Ready(Ok(msg));
                }
            }
        }
    }

    /// Read one message.
    ///
    /// This converts `ErrorResponse` to `Result::Err`.
    ///
    /// This consume `NoticeResponse` and `ParameterStatus`.
    pub fn read_msg(&mut self) -> Result<Message<'_>, Error> {
        loop {
            let Some(msg) = self.read_buf.read_msg()? else {
                if self.read_sync()? {
                    return Err("connection terminated".into());
                }
                continue;
            };
            match msg.ty() {
                B::ErrorResponse => return as_error(msg),
                B::NoticeResponse => {
                    println!("notice: {}", msg.decode::<backend::NoticeResponse>().unwrap());
                    continue;
                }
                B::ParameterStatus => continue, // TODO: pg: parameter status
                _ => {}
            }
            // SAFETY: impossible: https://users.rust-lang.org/t/mutably-borrowed-in-previous-iteration-despite-no-chance-of-conflicting-borrow/83274/3
            return Ok(unsafe { mem::transmute::<Message<'_>, Message<'_>>(msg) });
        }
    }

    /// Read and decode one message.
    ///
    /// This converts `ErrorResponse` to `Result::Err`.
    ///
    /// This consume `NoticeResponse` and `ParameterStatus`.
    pub fn decode_msg<'a, D: Decode<'a>>(&'a mut self) -> Result<D, Error> {
        Ok(self.read_msg()?.decode_checked()?)
    }
}

// ===== Write =====

impl Postgres {
    pub fn write_startup(&mut self, startup: &frontend::Startup) -> Result<(), OutOfCapacity> {
        startup.encode(&mut self.write_buf)
    }

    pub fn write_msg<M: Encode>(&mut self, msg: M) -> Result<(), OutOfCapacity> {
        msg.encode_msg(&mut self.write_buf)
    }

    pub fn flush_sync(&mut self) -> Result<(), WriteError> {
        self.write_buf.flush_sync(&self.socket)
    }

    pub fn poll_flush(&mut self, cx: &mut task::Context) -> Poll<Result<(), WriteError>> {
        self.write_buf.poll_flush(&self.socket, cx)
    }
}

impl fmt::Debug for Postgres {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Postgres").field(&self.socket).finish()
    }
}

// ===== helpers =====

fn as_error<T>(msg: Message) -> Result<T, Error> {
    println!("db error: {:#?}", msg.decode::<backend::ErrorResponse>().unwrap());
    Err(Error)
}
