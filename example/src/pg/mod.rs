use core::ffi::CStr;
use core::task;

use genos::net::socket::Socket;
use genos::net;
use seryu::postgres::backend::BackendType as B;
use seryu::postgres::{Decode, Encode, backend, frontend};

use crate::pg::read::ReadBuf;
use crate::pg::write::WriteBuf;
use crate::{Error, alloc};

mod read;
mod write;

pub struct Postgres {
    socket: Socket,
    read_buf: ReadBuf,
    write_buf: WriteBuf,
}

impl Postgres {
    pub fn connect(path: &CStr) -> Result<Self, Error> {
        // create socket
        let socket = Socket::create(Socket::UNIX, Socket::STREAM, <_>::default())?;
        let addr = net::SockAddrUn::from_path(path.into());
        socket.connect(addr.as_sockaddr(), addr.addrlen())?;

        // allocate message buffer
        let ptr = alloc::leak_slice(ReadBuf::CAPACITY + WriteBuf::CAPACITY);
        let read_buf = unsafe { ReadBuf::from_raw(ptr) };
        let write_buf = unsafe { WriteBuf::from_raw(ptr.add(ReadBuf::CAPACITY)) };

        Ok(Self { socket, read_buf, write_buf })
    }
}

impl Postgres {
    pub fn startup(&mut self) -> Result<(), Error> {
        // write to buffer
        frontend::Startup { user: "deuzo", database: None, replication: None }
            .encode(&mut self.write_buf)?;

        // write to io
        let mut cx = task::Context::from_waker(task::Waker::noop());
        let poll = self.write_buf.poll_flush(&self.socket, &mut cx)?;
        assert!(poll.is_ready(), "socket is not yet set to nonblocking");
        self.write_buf.clear();

        // read from io
        let poll = self.read_buf.poll_read_fd(&self.socket, &mut cx)?;
        assert!(poll.is_ready(), "socket is not yet set to nonblocking");

        // read from buffer
        loop {
            let message = backend::read_message(&mut self.read_buf)
                .ok_or("expected ready for query in startup")?;
            let (header, body) = message
                .split_first_chunk::<5>()
                .expect("TODO: built in wrapper type");
            let Some(ty) = B::from_raw(header[0]) else {
                return Err(format!("unknown backend msg: {:?}", header[0] as char).into());
            };
            match ty {
                B::ErrorResponse => {
                    let err = backend::ErrorResponse::decode(body)?;
                    return Err(format!("{err:#?}").into());
                }
                B::BackendKeyData => (), // TODO: pg: backend key data
                B::Authentication => {
                    let auth = backend::Authentication::decode(body)?;
                    match auth.kind() {
                        Some(backend::AuthenticationKind::Ok) => {}
                        _ => return Err("unsupported authentication kind".into()),
                    }
                }
                B::ParameterStatus => (), // TODO: pg: parameter status
                B::ReadyForQuery => break,
                ty => return Err(format!("unexpected message: {:?}", ty).into()),
            }
        }
        Ok(())
    }
}

impl Postgres {
    pub fn simple_query(&mut self, sql: &str) -> Result<(), Error> {
        // write to buffer
        frontend::Query { sql }.encode_msg(&mut self.write_buf)?;

        // write to io
        let mut cx = task::Context::from_waker(task::Waker::noop());
        let poll = self.write_buf.poll_flush(&self.socket, &mut cx)?;
        assert!(poll.is_ready(), "socket is not yet set to nonblocking");
        self.write_buf.clear();

        // read from io
        let poll = self.read_buf.poll_read_fd(&self.socket, &mut cx)?;
        assert!(poll.is_ready(), "socket is not yet set to nonblocking");

        // read from buffer
        loop {
            let message =
                backend::read_message(&mut self.read_buf).ok_or("expected ready for query")?;
            let (header, body) = message
                .split_first_chunk::<5>()
                .expect("TODO: built in wrapper type");
            let Some(ty) = B::from_raw(header[0]) else {
                return Err(format!("unknown backend msg: {:?}", header[0] as char).into());
            };
            match ty {
                B::CommandComplete => (),
                B::DataRow => {
                    for col in backend::DataRow::decode(body)? {
                        println!("{col:?}");
                    }
                }
                B::ErrorResponse => {
                    let err = backend::ErrorResponse::decode(body)?;
                    return Err(format!("{err:#?}").into());
                }
                B::RowDescription => {
                    for field in backend::RowDescription::decode(body)? {
                        println!("{field:?}");
                    }
                }
                B::ReadyForQuery => break,
                // CopyInResponse
                // CopyOutResponse
                // EmptyQueryResponse
                // NoticeResponse
                ty => return Err(format!("unexpected message: {:?}", ty).into()),
            }
        }

        Ok(())
    }
}

pub fn connect() -> Result<(), Error> {
    let mut conn = Postgres::connect(c"/run/postgresql/.s.PGSQL.5432")?;
    conn.startup()?;
    conn.simple_query("SELECT * FROM protocols")?;
    Ok(())
}
