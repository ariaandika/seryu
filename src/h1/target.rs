use crate::matches;
use crate::uri::UriError;

// ===== Origin =====

/// Origin form request target.
///
/// `Target` contains [path] and optional [query] component from URI.
///
/// `Target` is retrieved from `HTTP/1.1` request target in request line, or `:path` pseudo-header
/// in `HTTP/2.0`.
///
/// [path]: <https://www.rfc-editor.org/info/rfc3986/#section-3.3>
/// [query]: <https://www.rfc-editor.org/info/rfc3986/#section-3.4>
#[derive(Debug, Default)]
pub struct Origin<'a> {
    pub path: &'a [u8],
    pub query: Option<&'a [u8]>,
}

impl<'a> Origin<'a> {
    /// Parse request target as `origin-form`.
    ///
    /// Use [`Form::from_prefix`] to checks the target form.
    ///
    /// ```not_rust
    /// origin-form     = absolute-path [ "?" query ]
    /// ```
    #[inline]
    pub const fn parse(target: &'a [u8]) -> Result<Self, UriError> {
        let mut origin = Self { path: &[], query: None };
        match parse_origin(target, &mut origin) {
            Ok(()) => Ok(origin),
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
/// Use [`Form::from_prefix`] to checks the target form.
///
/// ```not_rust
/// origin-form     = absolute-path [ "?" query ]
/// ```
pub const fn parse_origin<'a>(target: &'a [u8], output: &mut Origin<'a>) -> Result<(), UriError> {
    // absolute-path    = 1*( "/" segment )
    // segment          = *pchar

    let Some((&prefix, mut bytes)) = target.split_first() else {
        return Err(UriError::Empty);
    };

    // "?key=value" are allowed, which is not prefixed with '/'
    if prefix != b'/' {
        loop {
            let Some((&byte, rest)) = bytes.split_first() else {
                output.path = target;
                return Ok(());
            };
            if !is_pchar_or_slash(byte) {
                break;
            }
            bytes = rest;
        }
    };

    let Some((&delim, rest)) = bytes.split_first() else {
        output.path = target;
        return Ok(());
    };
    if delim != b'?' {
        return Err(UriError::InvalidPath);
    }

    let query = bytes;
    bytes = rest;

    loop {
        let [byte, rest @ ..] = bytes else {
            output.query = Some(query);
            return Ok(());
        };
        if !is_query(*byte) {
            return Err(UriError::InvalidPath);
        }
        bytes = rest;
    }
}
