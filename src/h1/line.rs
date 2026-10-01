use core::hint;

use crate::bytes::Reader;
use crate::h1::{ParseError, matches};

/// Read a CRLF delimited line.
///
/// Advance `reader` to consume the line, and returns the line without its delimiter.
///
/// This function also accepts bare LF delimiter.
#[inline]
pub fn read_line<'a>(reader: &mut Reader<'a>) -> Result<&'a [u8], ParseError> {
    let Some(line) = matches::find::<b'\n'>(reader.as_bytes()) else {
        return Err(ParseError::InsufficientBuf);
    };
    reader.assume_read_len(line.len());
    let Some(&[delim]) = reader.read_chunk() else {
        // SAFETY: `find` never returns empty bytes
        unsafe { hint::unreachable_unchecked() }
    };
    if delim != b'\n' {
        // panic!("ffa");
        return Err(ParseError::InvalidByte);
    }
    let suffix = line.last().copied().unwrap_or(b'0') == b'\r';
    Ok(unsafe { line.get_unchecked(..line.len() - suffix as usize) })
}
