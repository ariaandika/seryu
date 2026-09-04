use crate::bytes::{InsufficientBuffer, Reader};
use crate::h1::matches;

/// Read a CRLF delimited line.
///
/// Advance `reader` to consume the line, and returns the line without its delimiter.
///
/// This function also accepts bare LF delimiter.
#[inline]
pub fn read_line<'a>(reader: &mut Reader<'a>) -> Result<&'a [u8], InsufficientBuffer> {
    let Some(line) = matches::find::<b'\n'>(reader.as_bytes()) else {
        return Err(InsufficientBuffer);
    };
    reader.assume_read_len(line.len() + 1);
    let suffix = line.last().copied().unwrap_or(b'\0') == b'\r';
    Ok(unsafe { line.get_unchecked(..line.len() - suffix as usize) })
}
