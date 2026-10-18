use core::ffi::CStr;

use genos::net;
use genos::net::Socket;
use seryu::postgres::backend::BackendType as B;
use seryu::postgres::{backend, frontend};

use crate::pg::read::ReadBuf;
use crate::pg::write::WriteBuf;
use crate::{Error, alloc};

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
                let is_eof = self.read_buf.read_fd_sync(&self.socket)?;
                if is_eof {
                    return Err("connection terminated".into());
                }
                continue;
            };
            let auth = match msg.ty() {
                B::ErrorResponse => return as_error(msg),
                B::NoticeResponse => on_notice!(msg),
                B::Authentication => msg.decode::<backend::Authentication>()?,
                ty => return Err(format!("unexpected message: {:?}", ty).into()),
            };
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
            let Some(msg) = self.read_buf.read_msg()? else {
                let is_eof = self.read_buf.read_fd_sync(&self.socket)?;
                if is_eof {
                    return Err("connection terminated".into());
                }
                continue;
            };
            match msg.ty() {
                B::ErrorResponse => return as_error(msg),
                B::BackendKeyData => (),  // TODO: pg: backend key data
                B::NoticeResponse => on_notice!(msg),
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
            let Some(msg) = self.read_buf.read_msg()? else {
                let is_eof = self.read_buf.read_fd_sync(&self.socket)?;
                if is_eof {
                    return Err("connection terminated".into());
                }
                continue;
            };
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
                B::NoticeResponse => on_notice!(msg),
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

// ===== helpers =====

fn as_error<T>(msg: backend::Message) -> Result<T, Error> {
    Err(msg.decode::<backend::ErrorResponse>().unwrap().into())
}

macro_rules! on_notice {
    ($msg:ident) => {{
        println!("notice: {}", $msg.decode::<backend::NoticeResponse>().unwrap());
        continue;
    }};
}
use on_notice;

// fn on_notice(msg: backend::Message) {
//     println!("notice: {}", msg.decode::<backend::NoticeResponse>().unwrap());
// }
