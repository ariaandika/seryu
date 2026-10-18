use core::ffi::CStr;
use core::fmt;

use genos::net;
use genos::net::Socket;
use seryu::postgres::backend::BackendType as B;
use seryu::postgres::{backend, frontend};

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

impl Postgres {
    pub fn startup(
        &mut self,
        user: &str,
        database: Option<&str>,
        replication: Option<&str>,
    ) -> Result<(), Error> {
        // send startup message
        frontend::Startup { user, database, replication }.encode(&mut self.write_buf)?;
        self.write_buf.flush_sync(&self.socket)?;

        // receive authentication negotiation
        loop {
            let Some(msg) = self.read_buf.read_msg()? else {
                // read from io
                self.read_buf.read_fd_sync(&self.socket)?;
                continue;
            };
            match msg.ty() {
                B::ErrorResponse => return as_error(msg),
                B::Authentication => {}
                ty => return Err(format!("unexpected message: {:?}", ty).into()),
            }
            let auth = msg.decode::<backend::Authentication>()?;
            match auth.kind() {
                Some(backend::AuthenticationKind::Ok) => break,
                // CleartextPassword    // password
                // GSS                  // gss
                // SSPI                 // sspi
                // SASL                 // scram-sha-256
                ty => return Err(format!("unsupported authentication kind: {ty:?}").into()),
            }
        }

        // receive remanining startup responses
        loop {
            let msg = self.read_buf.read_msg()?.ok_or(NoReady)?;
            match msg.ty() {
                B::ErrorResponse => return as_error(msg),
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
        self.write_buf.write_msg(frontend::Query { sql })?;
        self.write_buf.flush_sync(&self.socket)?;

        self.read_buf.read_fd_sync(&self.socket)?;
        loop {
            let msg = self.read_buf.read_msg()?.ok_or(NoReady)?;
            match msg.ty() {
                // CopyInResponse
                // CopyOutResponse
                B::CommandComplete => (),
                B::DataRow => {
                    for col in msg.decode::<backend::DataRow>()? {
                        println!("{col:?}");
                    }
                }
                B::ErrorResponse => return as_error(msg),
                B::EmptyQueryResponse => (),
                B::NoticeResponse => {
                    println!("pg notice: {}", msg.decode::<backend::NoticeResponse>()?)
                }
                B::RowDescription => {
                    for field in msg.decode::<backend::RowDescription>()? {
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

// ===== helpers =====

struct NoReady;

impl fmt::Display for NoReady {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expected ready for query")
    }
}

fn as_error<T>(msg: backend::Message) -> Result<T, Error> {
    Err(msg.decode::<backend::ErrorResponse>().unwrap().into())
}
