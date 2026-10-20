use seryu::postgres::{backend, frontend};

use crate::{Error, pg::Postgres};

pub fn query(pg: &mut Postgres) -> Result<(), Error> {
    let name = "lmao_stmt";
    let portal = "lmao_portal";

    pg.write_msg(frontend::Parse { name, query: "select * from ehh", params: &[] })?;
    pg.write_msg(frontend::Describe { kind: b'P', name })?;
    pg.write_msg(frontend::Bind { portal, name, formats: &[], params: &[], results: &[1] })?;
    pg.write_msg(frontend::Execute { portal, max_row: 0 })?;
    pg.write_msg(frontend::Sync)?;

    pg.flush_sync()?;

    pg.read_msg()?.decode_checked::<backend::ParseComplete>()?;

    let params = pg.read_msg()?.decode_checked::<backend::ParameterDescription>()?;
    println!("param oids: {:?}", params.into_iter());

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
            B::EmptyQueryResponse => break,
            B::PortalSuspended => todo!(),
            ty => return Err(format!("unexpected message: {:?}", ty).into()),
        }
    }

    pg.read_msg()?.decode_checked::<backend::ReadyForQuery>()?;

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
