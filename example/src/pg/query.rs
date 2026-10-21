use core::task::{self, Poll, ready};
use core::{mem, pin};

use seryu::postgres::{Columns, backend, frontend};

use crate::error::Error;
use crate::pg::Postgres;

// ===== Query =====

/// Postgres extended query.
#[derive(Debug)]
pub struct Query<'a> {
    name: &'a str,
    portal: &'a str,
    query: &'a str,
    phase: Phase,
    flush: bool,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Init,
    ParseRes,
    BindRes,
    DataRes,
    CloseRes,
    CloseSyncRes,
    Closed,
}

impl<'a> Query<'a> {
    pub fn new(name: &'a str, portal: &'a str, query: &'a str) -> Self {
        Self { name, portal, query, phase: Phase::Init, flush: false }
    }

    /// Fetch one column from the query.
    pub fn fetch<'p>(&mut self, conn: &'p mut Postgres) -> Fetch<'a, '_, 'p> {
        Fetch { state: self, conn }
    }
}

// ===== Fetch =====

pub struct Fetch<'a, 'q, 'c> {
    state: &'q mut Query<'a>,
    conn: &'c mut Postgres,
}

impl<'a, 'q, 'c> Future for Fetch<'a, 'q, 'c> {
    type Output = Result<Option<Columns<'c>>, Error>;

    fn poll(self: pin::Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let Self { state, conn } = self.get_mut();
        loop {
            if state.flush {
                ready!(conn.poll_flush(cx))?;
                state.flush = false;
            }
            match state.phase {
                Phase::Init => {
                    conn.write_msg(frontend::Parse {
                        name: state.name,
                        query: state.query,
                        params: &[],
                    })?;
                    conn.write_msg(frontend::Bind {
                        portal: state.portal,
                        name: state.name,
                        formats: &[],
                        params: &[],
                        results: &[1],
                    })?;
                    conn.write_msg(frontend::Execute { portal: state.portal, max_row: 0 })?;
                    conn.write_msg(frontend::Sync)?;
                    state.flush = true;
                    state.phase = Phase::ParseRes;
                }
                Phase::ParseRes => {
                    ready!(conn.poll_msg(cx))?.decode_checked::<backend::ParseComplete>()?;
                    state.phase = Phase::BindRes;
                }
                Phase::BindRes => {
                    ready!(conn.poll_msg(cx))?.decode_checked::<backend::BindComplete>()?;
                    state.phase = Phase::DataRes;
                }
                Phase::DataRes => {
                    loop {
                        use backend::BackendType as B;
                        let msg = ready!(conn.poll_msg(cx))?;
                        match msg.ty() {
                            B::CommandComplete => {}
                            B::DataRow => {
                                let msg = msg.decode::<backend::DataRow>()?;
                                // SAFETY: impossible
                                let cols = unsafe {
                                    mem::transmute::<Columns<'_>, Columns<'_>>(msg.into_iter())
                                };
                                return Poll::Ready(Ok(Some(cols)));
                            }
                            B::EmptyQueryResponse => {}
                            B::ReadyForQuery => break,
                            B::PortalSuspended => todo!(),
                            ty => {
                                return Poll::Ready(Err(
                                    format!("unexpected message: {:?}", ty).into()
                                ));
                            }
                        }
                    }
                    conn.write_msg(frontend::Close { kind: b'S', name: state.name })?;
                    conn.write_msg(frontend::Sync)?;
                    state.flush = true;
                    state.phase = Phase::CloseRes;
                }
                Phase::CloseRes => {
                    ready!(conn.poll_msg(cx))?.decode_checked::<backend::CloseComplete>()?;
                    state.phase = Phase::CloseSyncRes;
                }
                Phase::CloseSyncRes => {
                    ready!(conn.poll_msg(cx))?.decode_checked::<backend::ReadyForQuery>()?;
                    state.phase = Phase::Closed;
                    return Poll::Ready(Ok(None));
                }
                Phase::Closed => return Poll::Ready(Ok(None)),
            }
        }
    }
}

// ===== example =====

pub fn insert(pg: &mut Postgres) -> Result<(), Error> {
    let name = "lmao_stmt2";
    let portal = "lmao_portal";

    pg.write_msg(frontend::Parse {
        name,
        query: "insert into protocols values ($1, $2)",
        params: &[25, 23],
    })?;
    pg.write_msg(frontend::Bind {
        portal,
        name,
        formats: &[1, 1],
        params: &[b"websocket", &4u32.to_be_bytes()],
        results: &[],
    })?;
    pg.write_msg(frontend::Execute { portal, max_row: 0 })?;
    pg.write_msg(frontend::Sync)?;
    pg.flush_sync()?;

    pg.read_msg()?.decode_checked::<backend::ParseComplete>()?;
    pg.read_msg()?.decode_checked::<backend::BindComplete>()?;

    loop {
        use backend::BackendType as B;
        let msg = pg.read_msg()?;
        match msg.ty() {
            B::CommandComplete => break,
            B::DataRow => {
                for col in msg.decode::<backend::DataRow>()? {
                    println!("{col:?}");
                }
            }
            B::EmptyQueryResponse => (),
            B::PortalSuspended => todo!(),
            ty => return Err(format!("unexpected message: {:?}", ty).into()),
        }
    }

    pg.read_msg()?.decode_checked::<backend::ReadyForQuery>()?;

    Ok(())
}
