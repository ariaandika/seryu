use core::ffi::CStr;
use core::{fmt, task};

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
    #[allow(unused)]
    pub fn connect_unix(path: &CStr) -> Result<Self, Error> {
        let socket = Socket::create(Socket::UNIX, Socket::STREAM, <_>::default())?;
        let addr = net::SockAddrUn::from_path(path.into());
        socket.connect(addr.as_sockaddr(), addr.addrlen())?;
        Ok(Self::with_socket(socket))
    }

    #[allow(unused)]
    pub fn connect_tcp(addr: [u8; 4], port: u16) -> Result<Self, Error> {
        let socket = Socket::create(Socket::INET, Socket::STREAM, <_>::default())?;
        let addr = net::SockAddrIn::new(port.swap_bytes(), u32::from_ne_bytes(addr));
        socket.connect(addr.as_sockaddr(), addr.addrlen())?;
        Ok(Self::with_socket(socket))
    }

    pub fn with_socket(socket: Socket) -> Self {
        // allocate message buffer
        let ptr = alloc::leak_slice(ReadBuf::CAPACITY + WriteBuf::CAPACITY);
        let read_buf = unsafe { ReadBuf::from_raw(ptr) };
        let write_buf = unsafe { WriteBuf::from_raw(ptr.add(ReadBuf::CAPACITY)) };
        Self { socket, read_buf, write_buf }
    }
}

impl Postgres {
    pub fn startup(
        &mut self,
        user: &str,
        database: Option<&str>,
        replication: Option<&str>,
    ) -> Result<(), Error> {
        // write to buffer
        frontend::Startup { user, database, replication }.encode(&mut self.write_buf)?;

        // write to io
        let mut cx = task::Context::from_waker(task::Waker::noop());
        let poll = self.write_buf.poll_flush(&self.socket, &mut cx)?;
        assert!(poll.is_ready(), "socket is not yet set to nonblocking");

        // read authentication negotiation
        loop {
            let Some(msg) = backend::Message::read(&mut self.read_buf)? else {
                // read from io
                let poll = self.read_buf.poll_read_fd(&self.socket, &mut cx)?;
                assert!(poll.is_ready(), "socket is not yet set to nonblocking");
                continue;
            };
            match msg.ty() {
                B::ErrorResponse => {
                    let err = backend::ErrorResponse::decode(msg.body())?;
                    return Err(format!("{err:#?}").into());
                }
                B::Authentication => {}
                ty => return Err(format!("unexpected message: {:?}", ty).into()),
            }

            let auth = backend::Authentication::decode(msg.body())?;
            match auth.kind() {
                Some(backend::AuthenticationKind::Ok) => break,
                // CleartextPassword    // password
                // GSS                  // gss
                // SSPI                 // sspi
                // SASL                 // scram-sha-256
                ty => return Err(format!("unsupported authentication kind: {ty:?}").into()),
            }
        }

        // read remanining startup responses
        loop {
            let msg = backend::Message::read(&mut self.read_buf)?.ok_or(NoReady)?;
            match msg.ty() {
                B::ErrorResponse => {
                    let err = backend::ErrorResponse::decode(msg.body())?;
                    return Err(format!("{err:#?}").into());
                }
                B::BackendKeyData => (),  // TODO: pg: backend key data
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

        // read from io
        let poll = self.read_buf.poll_read_fd(&self.socket, &mut cx)?;
        assert!(poll.is_ready(), "socket is not yet set to nonblocking");

        // read from buffer
        loop {
            let msg = backend::Message::read(&mut self.read_buf)?.ok_or(NoReady)?;
            match msg.ty() {
                // CopyInResponse
                // CopyOutResponse
                B::CommandComplete => (),
                B::DataRow => {
                    for col in backend::DataRow::decode(msg.body())? {
                        println!("{col:?}");
                    }
                }
                B::ErrorResponse => {
                    let err = backend::ErrorResponse::decode(msg.body())?;
                    return Err(format!("{err:#?}").into());
                }
                B::EmptyQueryResponse => (),
                B::NoticeResponse => {
                    println!("pg notice: {}", backend::NoticeResponse::decode(msg.body())?)
                }
                B::RowDescription => {
                    for field in backend::RowDescription::decode(msg.body())? {
                        println!("{field:?}");
                    }
                }
                B::ReadyForQuery => break,
                ty => return Err(format!("unexpected message: {:?}", ty).into()),
            }
        }

        Ok(())
    }
}

pub fn connect() -> Result<(), Error> {
    // let mut conn = Postgres::connect_unix(c"/run/postgresql/.s.PGSQL.5432")?;
    let mut conn = Postgres::connect_tcp([127, 0, 0, 1], 5432)?;
    conn.startup("deuzo", None, None)?;
    conn.simple_query("SELECT * FROM protocols")?;
    Err(Error)
}

struct NoReady;

impl fmt::Display for NoReady {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expected ready for query")
    }
}
