//! Postgres frontend messages.
use core::ffi::CStr;

use crate::alloc::OutOfCapacity;
use crate::bytes::BufMut;
use crate::postgres::{Encode, Encoder, Primitive};

macro_rules! unit_msg {
    ($(
        $(#[doc = $doc:literal])*
        $vis:vis struct $name:ident($ty:literal);
    )*) => {$(
        $(#[doc = $doc])*
        #[derive(Debug)]
        $vis struct $name;

        impl Encode for $name {
            const MSGTYPE: u8 = $ty;
            #[inline]
            fn size(&self) -> usize { 0 }
            #[inline]
            unsafe fn encode_raw(&self, _: *mut u8) { }
        }
    )*};
}
unit_msg! {
    /// Flush command.
    pub struct Flush(b'H');

    /// Sync command.
    pub struct Sync(b'S');

    /// Termination.
    pub struct Terminate(b'X');
}

// ===== Bind =====

/// Bind command.
#[derive(Debug)]
pub struct Bind<'a> {
    /// Destination portal name.
    pub portal: &'a str,
    /// Prepared statement name.
    pub name: &'a str,
    /// Parameter format codes.
    pub formats: &'a [u16],
    /// Parameter values.
    pub params: &'a [&'a [u8]],
    /// Result-column format codes.
    pub results: &'a [u16],
}

impl Encode for Bind<'_> {
    const MSGTYPE: u8 = b'B';

    #[inline]
    fn size(&self) -> usize {
        self.portal.size()
            + self.name.size()
            + size_of::<u16>()
            + size_of_val(self.formats)
            + size_of::<u16>()
            + self
                .params
                .iter()
                .fold(0, |v, n| v + size_of::<u32>() + n.len())
            + size_of::<u16>()
            + size_of_val(self.results)
    }

    #[inline]
    unsafe fn encode_raw(&self, ptr: *mut u8) {
        let mut enc = Encoder::new(ptr);
        unsafe {
            enc.encode(self.portal);
            enc.encode(self.name);
            enc.encode(self.formats.len() as u16);
            for &format in self.formats {
                enc.encode(format);
            }
            enc.encode(self.params.len() as u16);
            for &param in self.params {
                enc.encode(param.len() as u32);
                enc.encode(param);
            }
            enc.encode(self.results.len() as u16);
            for &result in self.results {
                enc.encode(result);
            }
        }
    }
}

// ===== CancelRequest =====

/// Bind command.
#[derive(Debug)]
pub struct CancelRequest<'a> {
    pub pid: u32,
    /// Secret key for the target backend.
    pub key: &'a [u8],
}

impl<'a> CancelRequest<'a> {
    /// Encode startup message to given buffer.
    #[inline]
    pub fn encode<B: BufMut>(&self, mut buf: B) -> Result<(), OutOfCapacity> {
        let len = size_of::<u32>() + size_of::<u32>() + self.key.len();

        if buf.spare_mut().len() < len {
            return Err(OutOfCapacity);
        }

        unsafe {
            const CODE: u32 = {
                let msb = 1234;
                let lsb = 5678;
                (msb << u16::BITS) | lsb
            };

            let mut enc = Encoder::new(buf.spare_mut().as_mut_ptr());
            enc.encode(CODE);
            enc.encode(self.pid);
            enc.encode(self.key);
        }

        Ok(())
    }
}

// ===== Close =====

/// Close command.
#[derive(Debug)]
pub struct Close<'a> {
    /// 'S' to close a prepared statement; or 'P' to close a portal.
    pub kind: u8,
    /// The name of the prepared statement or portal to close.
    pub name: &'a str,
}

impl Encode for Close<'_> {
    const MSGTYPE: u8 = b'C';

    #[inline]
    fn size(&self) -> usize {
        self.kind.size() + self.name.size()
    }

    #[inline]
    unsafe fn encode_raw(&self, ptr: *mut u8) {
        let mut enc = Encoder::new(ptr);
        unsafe {
            enc.encode(self.kind);
            enc.encode(self.name);
        }
    }
}

// ===== Describe =====

/// Describe command.
#[derive(Debug)]
pub struct Describe<'a> {
    /// 'S' to describe a prepared statement; or 'P' to describe a portal.
    pub kind: u8,
    /// Prepared statement or portal name.
    pub name: &'a str,
}

impl Encode for Describe<'_> {
    const MSGTYPE: u8 = b'D';

    #[inline]
    fn size(&self) -> usize {
        self.kind.size() + self.name.size()
    }

    #[inline]
    unsafe fn encode_raw(&self, ptr: *mut u8) {
        let mut enc = Encoder::new(ptr);
        unsafe {
            enc.encode(self.kind);
            enc.encode(self.name);
        }
    }
}

// ===== Execute =====

/// Execute command.
#[derive(Debug)]
pub struct Execute<'a> {
    /// Destination portal name.
    pub portal: &'a str,
    /// Maximum number of rows to return, zero denotes “no limit”.
    pub max_row: u32,
}

impl Encode for Execute<'_> {
    const MSGTYPE: u8 = b'E';

    #[inline]
    fn size(&self) -> usize {
        self.portal.size() + self.max_row.size()
    }

    #[inline]
    unsafe fn encode_raw(&self, ptr: *mut u8) {
        let mut enc = Encoder::new(ptr);
        unsafe {
            enc.encode(self.portal);
            enc.encode(self.max_row);
        }
    }
}

// ===== Parse =====

/// Parse command.
#[derive(Debug)]
pub struct Parse<'a> {
    /// Prepared statement name.
    pub name: &'a str,
    /// Query string.
    pub query: &'a str,
    /// Object ID of the data type for each parameter.
    pub params: &'a [u32],
}

impl Encode for Parse<'_> {
    const MSGTYPE: u8 = b'P';

    #[inline]
    fn size(&self) -> usize {
        self.name.size() + self.query.size() + size_of::<u16>() + size_of_val(self.params)
    }

    #[inline]
    unsafe fn encode_raw(&self, ptr: *mut u8) {
        let mut enc = Encoder::new(ptr);
        unsafe {
            enc.encode(self.name);
            enc.encode(self.query);
            enc.encode(self.params.len() as u16);
            for &param in self.params {
                enc.encode(param);
            }
        }
    }
}

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
