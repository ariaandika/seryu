use core::task::{self, Poll, ready};
use core::{error, fmt, pin};

use seryu::alloc::OutOfCapacity;
use seryu::error::{ReadError, WriteError};
use seryu::postgres::{backend, frontend};

use crate::error::Error;
use crate::pg::Postgres;

// ===== Query =====

#[derive(Debug)]
pub struct Startup<'s, 'c> {
    startup: frontend::Startup<'s>,
    conn: &'c mut Postgres,
    phase: Phase,
    flush: bool,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Init,
    AuthRes,
    StartupRes,
    Complete,
}

impl<'s, 'c> Startup<'s, 'c> {
    pub fn new(
        user: &'s str,
        database: Option<&'s str>,
        replication: Option<&'s str>,
        conn: &'c mut Postgres,
    ) -> Self {
        let startup = frontend::Startup { user, database, replication };
        Self { startup, conn, phase: Phase::Init, flush: false }
    }
}

impl Future for Startup<'_, '_> {
    type Output = Result<(), StartupError>;

    fn poll(self: pin::Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        use StartupError as E;
        use backend::{AuthenticationKind as A, BackendType as B};

        let Self { startup, conn, phase, flush } = self.get_mut();
        loop {
            if *flush {
                ready!(conn.poll_flush(cx))?;
                *flush = false;
            }
            match *phase {
                Phase::Init => {
                    conn.write_startup(startup)?;
                    *flush = true;
                    *phase = Phase::AuthRes;
                }
                Phase::AuthRes => {
                    let auth = conn.decode_msg::<backend::Authentication>()?;
                    match auth.kind() {
                        Some(A::Ok) => {}
                        // Some(A::CleartextPassword) => todo!(),
                        // Some(A::GSS) => todo!(),
                        // Some(A::SSPI) => todo!(),
                        // Some(A::SASL) => todo!(),
                        ty => return Poll::Ready(Err(E::UnsupportedAuth(ty))),
                    }
                    *phase = Phase::StartupRes;
                }
                Phase::StartupRes => {
                    loop {
                        let msg = conn.read_msg()?;
                        match msg.ty() {
                            B::BackendKeyData => (),  // TODO: pg: backend key data
                            B::ParameterStatus => (), // TODO: pg: parameter status
                            B::ReadyForQuery => break,
                            ty => return Poll::Ready(Err(E::UnexpectedMsg(ty))),
                        }
                    }
                    *phase = Phase::Complete;
                }
                Phase::Complete => return Poll::Ready(Ok(())),
            }
        }
    }
}

// ===== Error =====

#[derive(Debug)]
pub enum StartupError {
    UnsupportedAuth(Option<backend::AuthenticationKind>),
    UnexpectedMsg(backend::BackendType),
    Read(ReadError),
    Write(WriteError),
    OutOfCapacity,
    Error,
}

impl error::Error for StartupError {}

impl fmt::Display for StartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedAuth(auth) => {
                write!(f, "unsupported startup authentication")?;
                if let Some(auth) = auth {
                    write!(f, ": {auth:?}")?;
                }
                Ok(())
            }
            Self::UnexpectedMsg(ty) => write!(f, "unexpected message in startup: {ty:?}"),
            Self::Read(err) => err.fmt(f),
            Self::Write(err) => err.fmt(f),
            Self::OutOfCapacity => OutOfCapacity.fmt(f),
            Self::Error => Ok(()),
        }
    }
}

impl From<Error> for StartupError {
    fn from(_: Error) -> Self {
        Self::Error
    }
}

impl From<ReadError> for StartupError {
    fn from(err: ReadError) -> Self {
        Self::Read(err)
    }
}

impl From<WriteError> for StartupError {
    fn from(err: WriteError) -> Self {
        Self::Write(err)
    }
}

impl From<OutOfCapacity> for StartupError {
    fn from(_: OutOfCapacity) -> Self {
        Self::OutOfCapacity
    }
}
