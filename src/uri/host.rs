use crate::uri::{UriError, validate};

// ===== RegName =====

/// Registered Host Name.
///
/// ABNF: `reg-name = *( unreserved / pct-encoded / sub-delims )`
#[derive(Debug)]
pub struct RegName<'a>(&'a [u8]);

impl<'a> RegName<'a> {
    /// Create and validate [`RegName`] from raw bytes.
    ///
    /// Note that this method allows empty host.
    #[inline]
    pub const fn from_bytes(regname: &'a [u8]) -> Result<Self, UriError> {
        match validate::regname(regname) {
            Ok(()) => Ok(Self(regname)),
            Err(err) => Err(err),
        }
    }

    /// Returns the bytes representation.
    #[inline]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.0
    }
}
