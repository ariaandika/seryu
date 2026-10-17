//! Postgres frontend messages.
use core::ffi::CStr;

use crate::alloc::OutOfCapacity;
use crate::bytes::BufMut;
use crate::postgres::{Encode, Encoder, Primitive};

// ===== Query =====

/// Identifies the message as a simple query.
#[derive(Debug)]
pub struct Query<'a> {
    /// the query string itself
    pub sql: &'a str,
}

impl Encode for Query<'_> {
    const MSGTYPE: u8 = b'Q';

    #[inline]
    fn size(&self) -> usize {
        self.sql.size()
    }

    #[inline]
    unsafe fn encode_raw(&self, ptr: *mut u8) {
        unsafe { Encoder::once(ptr, self.sql) };
    }
}

// ===== SASLInitialResponse =====

/// Initial SASL response.
#[derive(Debug)]
pub struct SASLInitialResponse<'a> {
    /// Name of the SASL authentication mechanism that the client selected.
    pub name: &'a CStr,
    /// Length of SASL mechanism specific "Initial Client Response" that follows, or -1 if there is
    /// no Initial Response.
    pub length: u32,
    /// SASL mechanism specific "Initial Response".
    pub init_response: &'a [u8],
}

impl Encode for SASLInitialResponse<'_> {
    const MSGTYPE: u8 = b'p';

    #[inline]
    fn size(&self) -> usize {
        self.name.size() + self.length.size() + self.init_response.size()
    }

    #[inline]
    unsafe fn encode_raw(&self, ptr: *mut u8) {
        let mut enc = Encoder::new(ptr);
        unsafe {
            enc.encode(self.name);
            enc.encode(self.length);
            enc.encode(self.init_response);
        }
    }
}

// ===== SASLResponse =====

/// SASL response.
#[derive(Debug)]
pub struct SASLResponse<'a> {
    pub data: &'a [u8],
}

impl Encode for SASLResponse<'_> {
    const MSGTYPE: u8 = b'p';

    #[inline]
    fn size(&self) -> usize {
        self.data.size()
    }

    #[inline]
    unsafe fn encode_raw(&self, ptr: *mut u8) {
        unsafe { Encoder::once(ptr, self.data) };
    }
}

// ===== Startup =====

/// Connection startup message.
///
/// For historical reason, this message does not have message type, thus does not implements
/// [`Encode`]. Use [`Startup::encode`] instead.
#[derive(Debug)]
pub struct Startup<'a> {
    /// The database user name to connect as.
    pub user: &'a str,
    /// The database to connect to. Defaults to the user name.
    pub database: Option<&'a str>,
    /// Used to connect in streaming replication mode, where a small set of replication commands can
    /// be issued instead of SQL statements.
    ///
    /// Value can be true, false, or database, and the default is false.
    pub replication: Option<&'a str>,
}

impl<'a> Startup<'a> {
    /// Encode startup message to given buffer.
    pub fn encode<B: BufMut>(&self, mut buf: B) -> Result<(), OutOfCapacity> {
        let len = size_of::<u32>()
            + size_of::<u32>()
            + 7 // "user" + nul + `user` nul + message nul
            + self.user.len()
            + self.database.map(|e| "database".len() + e.len() + 2).unwrap_or(0)
            + self.replication.map(|s| "replication".len() + s.len() + 2).unwrap_or(0);

        if buf.spare_mut().len() < len {
            return Err(OutOfCapacity);
        }

        // https://www.postgresql.org/docs/current/protocol-message-formats.html#PROTOCOL-MESSAGE-FORMATS-STARTUPMESSAGE
        unsafe {
            const VERSION: u32 = {
                let major = 3u16.to_be_bytes();
                let minor = 2u16.to_be_bytes();
                u32::from_be_bytes([major[0], major[1], minor[0], minor[1]])
            };

            let mut enc = Encoder::new(buf.spare_mut().as_mut_ptr());

            enc.encode(len as u32);
            enc.encode(VERSION);
            enc.encode(c"user");
            enc.encode(self.user);

            if let Some(db) = self.database {
                enc.encode(c"database");
                enc.encode(db);
            }

            if let Some(repl) = self.replication {
                enc.encode(c"replication");
                enc.encode(repl);
            }

            // TODO: additional postgres startup parameter

            // In addition to the above, other parameters may be listed.
            // Parameter names beginning with _pq_. are reserved for use as protocol extensions,
            // while others are treated as run-time parameters to be set at backend start time.
            // Such settings will be applied during backend start
            // (after parsing the command-line arguments if any) and will act as session defaults.

            // A zero byte is required as a terminator after the last name/value pair.
            enc.encode(0u8);

            buf.advance_mut(len);
            Ok(())
        }
    }
}
