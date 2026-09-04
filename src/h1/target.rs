//! `HTTP/1.1` Message Request Target.
//!
//! There are four distinct formats for the `request-target`, depending on both the method being
//! requested and whether the request is to a proxy.
//!
//! No whitespace is allowed in the `request-target`.
//!
//! ```not_rust
//! request-target = origin-form
//!                / absolute-form
//!                / authority-form
//!                / asterisk-form
//! ```
//!
//! ## Origin Form
//!
//! When making a request directly to an origin server, other than a CONNECT or server-wide OPTIONS
//! request, a client MUST send only the absolute path and query components of the target URI as the
//! `request-target`. If the target URI's path component is empty, the client MUST send "/" as the
//! path within the `origin-form` of `request-target`.
//!
//! ```not_rust
//! origin-form    = absolute-path [ "?" query ]
//! ```
//!
//! ## Absolute Form
//!
//! When making a request to a proxy, other than a `CONNECT` or server-wide `OPTIONS` request, a
//! client MUST send the target URI in `absolute-form` as the `request-target`.
//!
//! ```not_rust
//! absolute-form  = absolute-URI
//! ```
//!
//! A server MUST accept the `absolute-form` in requests even though most `HTTP/1.1` clients will
//! only send the `absolute-form` to a proxy.
//!
//! ## Authority Form
//!
//! The `authority-form` of `request-target` is only used for CONNECT requests.
//!
//! ```not_rust
//! authority-form = uri-host ":" port
//! ```
//!
//! ## Asterisk Form
//!
//! The `asterisk-form` of `request-target` is only used for a server-wide OPTIONS request.
//!
//! ```not_rust
//! asterisk-form  = "*"
//! ```
use core::mem::MaybeUninit;

use crate::matches;
use crate::uri::UriError;

// ===== Origin =====

/// Origin Form Request Target.
///
/// Origin form contains absolute path and query components of the target URI.
///
/// ```not_rust
/// origin-form    = absolute-path [ "?" query ]
/// ```
#[derive(Debug, Default)]
pub struct Origin<'a> {
    /// Target path.
    pub path: &'a [u8],
    /// Target query.
    pub query: Option<&'a [u8]>,
}

impl<'a> Origin<'a> {
    /// Parse request target as `origin-form`.
    ///
    /// See the struct documentation for more details on the syntax.
    #[inline]
    pub const fn parse(target: &'a [u8]) -> Result<Self, UriError> {
        let mut origin = MaybeUninit::uninit();
        match parse_origin(target, &mut origin) {
            // SAFETY: `parse_origin` guarantee that `origin` is initialized
            Ok(()) => Ok(unsafe { origin.assume_init() }),
            Err(err) => Err(err),
        }
    }
}

// ===== parsers =====

matches::ascii_lookup_table! {
    /// `pchar            = unreserved / pct-encoded / sub-delims / ":" / "@"`
    /// `pchar-and-slash  = pchar / "/"`
    const fn is_pchar_or_slash(byte: u8) -> bool {
        matches::unreserved(byte)
        || matches::pct_encoded(byte)
        || matches::sub_delims(byte)
        || matches!(byte, b':' | b'@')
        || matches!(byte, b'/')
    }
}

matches::ascii_lookup_table! {
    /// `query = *( pchar / "/" / "?" )`
    const fn is_query(byte: u8) -> bool {
        is_pchar_or_slash(byte)
        || matches!(byte, b'?')
    }
}

/// Parse request target as `origin-form`.
///
/// See [`Origin`] for more details on the syntax.
pub const fn parse_origin<'a>(
    target: &'a [u8],
    output: &mut MaybeUninit<Origin<'a>>,
) -> Result<(), UriError> {
    // origin-form      = absolute-path [ "?" query ]
    // absolute-path    = 1*( "/" segment )
    // segment          = *pchar

    let Some((prefix, mut bytes)) = target.split_first() else {
        return Err(UriError::Empty);
    };
    if *prefix != b'/' {
        return Err(UriError::InvalidPath);
    }

    loop {
        let Some((byte, rest)) = bytes.split_first() else {
            output.write(Origin { path: target, query: None });
            return Ok(());
        };
        if !is_pchar_or_slash(*byte) {
            break;
        }
        bytes = rest;
    }

    let Some((delim, mut bytes)) = bytes.split_first() else {
        output.write(Origin { path: target, query: None });
        return Ok(());
    };
    if *delim != b'?' {
        return Err(UriError::InvalidPath);
    }

    // SAFETY: `delim` is `Some` result of `bytes.split_first()`
    let (path, query) = unsafe {
        target.split_at_unchecked((delim as *const u8).offset_from_unsigned(target.as_ptr()))
    };
    let origin = Origin { path, query: Some(query) };

    loop {
        let [byte, rest @ ..] = bytes else {
            output.write(origin);
            return Ok(());
        };
        if !is_query(*byte) {
            return Err(UriError::InvalidPath);
        }
        bytes = rest;
    }
}
