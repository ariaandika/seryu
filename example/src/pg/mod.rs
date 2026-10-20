pub use postgres::Postgres;

use crate::Error;

mod read;
mod write;

mod postgres;
mod query;

pub fn connect() -> Result<(), Error> {
    // let mut conn = Postgres::connect_unix(c"/run/postgresql/.s.PGSQL.5432")?;
    let mut conn = Postgres::connect_tcp([127, 0, 0, 1], 5432)?;
    conn.startup("deuzo", None, None)?;
    println!("startup ok");
    let res = conn.simple_query("SELECT * FROM protocols");
    println!("simple query: {res:?}");
    let res = query::query(&mut conn);
    println!("extend query: {res:?}");
    if res.is_err() {
        conn.drain_to_sync()?;
    }
    let res = query::insert(&mut conn);
    println!("insert query: {res:?}");
    Err(Error)
}
