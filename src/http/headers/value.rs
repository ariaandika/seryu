use core::fmt;

use crate::http::headers::error::HeaderError;
use crate::http::headers::matches;

/// HTTP Header Value.
///
/// This API does not support non-ASCII value.
#[repr(transparent)]
pub struct HeaderValue([u8]);

impl HeaderValue {
    pub(crate) const MAX_LENGTH: usize = 8 * 1024; // 8KB

    /// Parse header value from raw bytes.
    ///
    /// # Errors
    ///
    /// Returns error if the input is not a valid header value.
    #[inline]
    pub const fn from_bytes(value: &[u8]) -> Result<&Self, HeaderError> {
        match validate_header_value(value) {
            Ok(()) => Ok(Self::from_raw(value)),
            Err(err) => Err(err),
        }
    }

    const fn from_raw(name: &[u8]) -> &Self {
        unsafe { &*(name as *const _ as *const _) }
    }

    /// Returns the bytes representation.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { &*(self as *const _ as *const _) }
    }

    /// Returns the string representation.
    ///
    /// The returned string will always be in ASCII lowercase.
    #[inline]
    pub const fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }
}

// ===== Parsing =====

const fn validate_header_value(mut bytes: &[u8]) -> Result<(), HeaderError> {
    use HeaderError as E;
    match bytes {
        // no leading SP / HTAB
        | [b' ' | b'\t', ..]
        // no trailing SP / HTAB
        | [.., b' ' | b'\t'] => {
            return Err(E::Invalid);
        },
        _ => {}
    }
    // too long
    if bytes.len() > HeaderValue::MAX_LENGTH {
        return Err(E::ExcessiveBytes);
    }
    loop {
        let [byte, rest @ ..] = bytes else {
            return Ok(());
        };
        if !matches::is_header_value(*byte) {
            return Err(E::Invalid);
        }
        bytes = rest;
    }
}

// ===== Traits =====

impl fmt::Debug for HeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("HeaderValue").field(&self.as_str()).finish()
    }
}

impl PartialEq for HeaderValue {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<[u8]> for HeaderValue {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.as_bytes() == other
    }
}

impl PartialEq<str> for HeaderValue {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}
