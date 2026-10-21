use core::mem;

use seryu::postgres::{Columns, backend, frontend};

use crate::Error;
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
    Closing,
    Closed,
}

impl<'a> Query<'a> {
    pub fn new(name: &'a str, portal: &'a str, query: &'a str) -> Self {
        Self { name, portal, query, phase: Phase::Init, flush: false }
    }

    /// Fetch one column from the query.
    pub fn fetch<'p>(&mut self, conn: &'p mut Postgres) -> Result<Option<Columns<'p>>, Error> {
        loop {
            if self.flush {
                conn.flush_sync()?;
                self.flush = false;
            }
            match self.phase {
                Phase::Init => {
                    conn.write_msg(frontend::Parse {
                        name: self.name,
                        query: self.query,
                        params: &[],
                    })?;
                    conn.write_msg(frontend::Bind {
                        portal: self.portal,
                        name: self.name,
                        formats: &[],
                        params: &[],
                        results: &[1],
                    })?;
                    conn.write_msg(frontend::Execute { portal: self.portal, max_row: 0 })?;
                    conn.write_msg(frontend::Sync)?;
                    self.flush = true;
                    self.phase = Phase::ParseRes;
                }
                Phase::ParseRes => {
                    conn.decode_msg::<backend::ParseComplete>()?;
                    self.phase = Phase::BindRes;
                }
                Phase::BindRes => {
                    conn.decode_msg::<backend::BindComplete>()?;
                    self.phase = Phase::DataRes;
                }
                Phase::DataRes => {
                    loop {
                        use backend::BackendType as B;
                        let msg = conn.read_msg()?;
                        match msg.ty() {
                            B::CommandComplete => {}
                            B::DataRow => {
                                let msg = msg.decode::<backend::DataRow>()?;
                                // SAFETY: impossible
                                let cols = unsafe {
                                    mem::transmute::<Columns<'_>, Columns<'_>>(msg.into_iter())
                                };
                                return Ok(Some(cols));
                            }
                            B::EmptyQueryResponse => {}
                            B::ReadyForQuery => break,
                            B::PortalSuspended => todo!(),
                            ty => return Err(format!("unexpected message: {:?}", ty).into()),
                        }
                    }
                    conn.write_msg(frontend::Close { kind: b'S', name: self.name })?;
                    conn.write_msg(frontend::Sync)?;
                    self.flush = true;
                    self.phase = Phase::Closing;
                }
                Phase::Closing => {
                    // NOTE: may pending
                    conn.decode_msg::<backend::CloseComplete>()?;
                    conn.decode_msg::<backend::ReadyForQuery>()?;
                    self.phase = Phase::Closed;
                    return Ok(None);
                }
                Phase::Closed => {
                    return Ok(None);
                }
            }
        }
    }
}

// ===== example =====

pub fn query(conn: &mut Postgres) -> Result<(), Error> {
    let mut query = Query::new("stmt", "portal", "select * from protocols");

    while let Some(col) = query.fetch(conn)? {
        println!("{col:?}");
    }

    Ok(())
}

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
