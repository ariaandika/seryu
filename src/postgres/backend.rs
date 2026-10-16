//! Postgres backend messages.
use core::ffi::CStr;

use crate::bytes::Buf;
use crate::postgres::row::Rows;
use crate::postgres::{Column, Columns, Decode, DecodeError, Decoder, Row};
use crate::{fmt, matches};

// ===== message =====

/// Read one message.
#[inline]
pub fn read_message<'a, B: Buf<'a>>(mut bytes: B) -> Option<&'a [u8]> {
    const MSGTYPE_SIZE: u32 = size_of::<u8>() as u32;
    let len_bytes = bytes.as_bytes().first_chunk::<5>()?.last_chunk::<4>()?;
    let len = u32::from_be_bytes(*len_bytes) + MSGTYPE_SIZE;
    let message = bytes.as_bytes().get(..len as usize)?;
    bytes.advance(message.len());
    Some(message)
}

macro_rules! msgtype {
    ($($ty:ident,)*) => {
        /// Postgres backend message type.
        #[derive(Debug, Copy, Clone)]
        #[repr(u8)]
        pub enum BackendType {
            $($ty = $ty::MSGTYPE,)*
        }

        impl BackendType {
            /// Creates [`BackendType`] from raw value.
            #[inline]
            pub const fn from_raw(msgtype: u8) -> Option<Self> {
                match msgtype {
                    $($ty::MSGTYPE => Some(Self::$ty),)*
                    _ => None,
                }
            }
        }
    };
}

msgtype! {
    CommandComplete,
    DataRow,
    ErrorResponse,
    BackendKeyData,
    Authentication,
    ParameterStatus,
    RowDescription,
    ReadyForQuery,
}

struct ParameterStatus;

impl ParameterStatus {
    const MSGTYPE: u8 = b'S';
}

struct BackendKeyData;

impl BackendKeyData {
    const MSGTYPE: u8 = b'K';
}

// ===== Authentication =====

/// Authentication response kind.
#[derive(Debug)]
pub enum AuthenticationKind {
    Ok = 0,
    KerberosV5 = 2,
    CleartextPassword = 3,
    MD5Password = 5,
    GSS = 7,
    GSSContinue = 8,
    SSPI = 9,
    SASL = 10,
    SASLContinue = 11,
    SASLFinal = 12,
}

impl AuthenticationKind {
    /// Returns `true` if the authentication kind is [`Ok`].
    ///
    /// [`Ok`]: AuthenticationKind::Ok
    #[must_use]
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
}

/// Generic authentication response.
#[derive(Debug)]
pub struct Authentication {
    kind: u32,
    // ..: [u8]
}

impl Authentication {
    /// Returns the authentication response kind.
    ///
    /// Returns `None` if authentication response is unknown.
    #[inline]
    pub fn kind(&self) -> Option<AuthenticationKind> {
        use AuthenticationKind as K;
        match self.kind {
            0 => Some(K::Ok),
            2 => Some(K::KerberosV5),
            3 => Some(K::CleartextPassword),
            5 => Some(K::MD5Password),
            7 => Some(K::GSS),
            8 => Some(K::GSSContinue),
            9 => Some(K::SSPI),
            10 => Some(K::SASL),
            11 => Some(K::SASLContinue),
            12 => Some(K::SASLFinal),
            _ => None,
        }
    }
}

impl Decode<'_> for Authentication {
    const MSGTYPE: u8 = b'R';

    #[inline]
    fn decode(body: &[u8]) -> Result<Self, DecodeError> {
        let mut dec = Decoder::new(body);
        Ok(Self { kind: dec.decode_sized()? })
    }
}

/// Successful authentication request.
#[derive(Debug)]
pub struct AuthenticationOk {
    // success: 0,
}

impl Decode<'_> for AuthenticationOk {
    const MSGTYPE: u8 = b'R';

    #[inline]
    fn decode(body: &[u8]) -> Result<Self, DecodeError> {
        if Decoder::once::<u32, _>(body)? == 0 { Ok(Self {}) } else { Err(DecodeError::Invalid) }
    }
}

// ===== CommandComplete =====

/// Command-completed response message.
#[derive(Debug)]
pub struct CommandComplete<'a> {
    pub tag: &'a CStr,
}

impl<'a> Decode<'a> for CommandComplete<'a> {
    const MSGTYPE: u8 = b'C';

    #[inline]
    fn decode(body: &'a [u8]) -> Result<Self, DecodeError> {
        Decoder::cstr(body).map(|tag| Self { tag })
    }
}

// ===== DataRow =====

/// Data row message.
#[derive(Debug)]
pub struct DataRow<'a> {
    len: u16,
    body: &'a [u8],
}

impl<'a> DataRow<'a> {
    /// Returns the number of columns.
    #[inline]
    pub fn len(&self) -> u16 {
        self.len
    }

    /// Returns the columns as iterator.
    #[inline]
    pub fn iter(&self) -> Columns<'a> {
        Columns::new(self.len, self.body)
    }
}

impl<'a> IntoIterator for DataRow<'a> {
    type Item = Column<'a>;

    type IntoIter = Columns<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Columns::new(self.len, self.body)
    }
}

impl<'a> Decode<'a> for DataRow<'a> {
    const MSGTYPE: u8 = b'D';

    #[inline]
    fn decode(body: &'a [u8]) -> Result<Self, DecodeError> {
        let mut dec = Decoder::new(body);
        Ok(Self { len: dec.decode_sized()?, body: dec.remaining() })
    }
}

// ===== RowDescription =====

/// Row description message.
#[derive(Debug)]
pub struct RowDescription<'a> {
    len: u16,
    body: &'a [u8],
}

impl<'a> RowDescription<'a> {
    /// Returns the number of rows.
    #[inline]
    pub fn len(&self) -> u16 {
        self.len
    }

    /// Returns the rows as iterator.
    #[inline]
    pub fn iter(&self) -> Rows<'a> {
        Rows::new(self.len, self.body)
    }
}

impl<'a> IntoIterator for RowDescription<'a> {
    type Item = Row<'a>;

    type IntoIter = Rows<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Rows::new(self.len, self.body)
    }
}

impl<'a> Decode<'a> for RowDescription<'a> {
    const MSGTYPE: u8 = b'T';

    #[inline]
    fn decode(body: &'a [u8]) -> Result<Self, DecodeError> {
        let mut dec = Decoder::new(body);
        Ok(Self { len: dec.decode_sized()?, body: dec.remaining() })
    }
}

// ===== ErrorResponse =====

/// Error response.
pub struct ErrorResponse<'a> {
    body: &'a [u8],
}

impl<'a> Decode<'a> for ErrorResponse<'a> {
    const MSGTYPE: u8 = b'E';

    #[inline]
    fn decode(body: &'a [u8]) -> Result<Self, DecodeError> {
        Ok(Self { body })
    }
}

// ===== ReadyForQuery  =====

/// Backend is ready for a new query cycle.
#[derive(Debug)]
pub struct ReadyForQuery {
    /// Current backend transaction status indicator.
    ///
    /// Possible values are 'I' if idle (not in a transaction block); 'T' if in a transaction block;
    /// or 'E' if in a failed transaction block (queries will be rejected until block is ended).
    pub status: u8,
}

impl Decode<'_> for ReadyForQuery {
    const MSGTYPE: u8 = b'Z';

    #[inline]
    fn decode(body: &[u8]) -> Result<Self, DecodeError> {
        Decoder::once(body).map(|status| Self { status })
    }
}

// ===== Error =====

impl<'a> fmt::Display for ErrorResponse<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut bytes = self.body;
        while let Some((ty, rest)) = bytes.split_first()
            && let Some((cstr, rest)) =
                matches::find_nul(rest).and_then(|b| rest.split_at_checked(b.len() + 1))
        {
            bytes = rest;
            if *ty != b'M' {
                continue;
            }
            let Some((_, s)) = cstr.split_last() else {
                break;
            };
            return match str::from_utf8(s) {
                Ok(s) => s.fmt(f),
                Err(err) => match s.get(..err.valid_up_to()) {
                    Some(s) => unsafe { str::from_utf8_unchecked(s) }.fmt(f),
                    None => Ok(()),
                },
            };
        }
        Ok(())
    }
}

impl<'a> fmt::Debug for ErrorResponse<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // https://www.postgresql.org/docs/current/protocol-error-fields.html
        let mut d = f.debug_struct("ErrorResponse");
        let mut bytes = self.body;

        while let Some((ty, rest)) = bytes.split_first()
            && *ty != 0
            && let Some((cstr, rest)) =
                matches::find_nul(rest).and_then(|b| rest.split_at_checked(b.len() + 1))
        {
            bytes = rest;
            let name = match ty {
                b'S' => "severity_local",
                b'V' => "severity",
                b'C' => "code",
                b'M' => "message",
                b'D' => "detail",
                b'H' => "hint",
                b'P' => "position",
                b'p' => "internal_position",
                b'q' => "internal_query",
                b'W' => "where",
                b's' => "schema_name",
                b't' => "table_name",
                b'c' => "column_name",
                b'd' => "data_type_name",
                b'n' => "constraint_name",
                b'F' => "file",
                b'L' => "line",
                b'R' => "routine",
                _ => "..",
            };
            let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(cstr) };
            d.field(name, &cstr);
        }
        d.finish()
    }
}
