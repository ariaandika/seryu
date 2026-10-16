use crate::bytes::Buf;
use crate::h1::matches;

/// Read a delimited line.
///
/// This will search for CRLF delimiter, move the bytes pointer past the line with the delimiter,
/// and returns that line without the delimiter.
///
/// This also accept bare LF delimiter.
///
/// Additionally, if non-ASCII byte is found, it will be treated as delimiter. This is intended to
/// fail at the parsing phase, instead of checks for either invalid byte or insufficient byte.
#[inline]
pub fn read_line<'a, B: Buf<'a>>(mut bytes: B) -> Option<&'a [u8]> {
    let line = matches::find::<b'\n'>(bytes.as_bytes())?;
    bytes.advance(line.len() + 1);
    unsafe {
        // checks for potential CR delimiter
        let cr = line.as_ptr().add(line.len().saturating_sub(1));
        // SAFETY: if `line.len()` is `0`, `cr` will deref the delimiter
        Some(line.get_unchecked(..line.len() - (*cr == b'\r') as usize))
    }
}
