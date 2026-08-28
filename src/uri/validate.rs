use crate::matches;
use crate::uri::UriError;

matches::ascii_lookup_table! {
    /// `reg-name = *( unreserved / pct-encoded / sub-delims )`
    const fn is_regname(byte: u8) -> bool {
        matches::unreserved(byte)
        || matches::pct_encoded(byte)
        || matches::sub_delims(byte)
    }
}

/// `reg-name   = *( unreserved / pct-encoded / sub-delims )`
pub(crate) const fn regname(host: &[u8]) -> Result<(), UriError> {
    let mut bytes = host;
    while let [byte, rest @ ..] = bytes {
        if !is_regname(*byte) {
            return Err(UriError::InvalidAuthority);
        }
        bytes = rest;
    }
    Ok(())
}
