use core::{pin, task};

pub use postgres::Postgres;

use crate::error::Error;

mod read;
mod write;

mod postgres;
mod startup;
mod query;

pub fn connect() -> Result<(), Error> {
    let mut cx = task::Context::from_waker(task::Waker::noop());
    let task::Poll::Ready(_) = pin::pin!(connect_inner()).poll(&mut cx) else {
        unreachable!("socket was set to non-blocking")
    };
    Err(Error)
}

async fn connect_inner() -> Result<(), Error> {
    // let mut conn = Postgres::connect_unix(c"/run/postgresql/.s.PGSQL.5432")?;
    let mut conn = Postgres::connect_tcp([127, 0, 0, 1], 5432)?;

    startup::Startup::new("deuzo", None, None, &mut conn).await?;

    let res = query::query(&mut conn);
    println!("query 1: {res:?}");
    if res.is_err() {
        conn.drain_to_sync()?;
    }

    let res = query::query(&mut conn);
    println!("query 2: {res:?}");
    if res.is_err() {
        conn.drain_to_sync()?;
    }

    let res = query::insert(&mut conn);
    println!("insert query: {res:?}");

    Ok(())
}
