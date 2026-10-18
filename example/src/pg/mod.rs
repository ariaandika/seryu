pub use postgres::Postgres;

use crate::Error;

mod read;
mod write;

mod postgres;

pub fn connect() -> Result<(), Error> {
    // let mut conn = Postgres::connect_unix(c"/run/postgresql/.s.PGSQL.5432")?;
    let mut conn = Postgres::connect_tcp([127, 0, 0, 1], 5432)?;
    conn.startup("deuzo", None, None)?;
    conn.simple_query("SELECT * FROM protocols")?;
    Err(Error)
}
