use crate::bytes::{InsufficientBuffer, Reader};

// ===== Search =====

/// Bytes searching implementation.
///
/// Use [`DefaultSearch`] to use default sequential searching.
///
/// # Safety
///
/// [`Search::find`] must returns index within bounds of `bytes`.
pub unsafe trait Search {
    /// Find `byte` in `bytes` and returns the index.
    fn find(bytes: &[u8], byte: u8) -> Option<usize>;

    #[inline]
    fn find_as_bytes(bytes: &[u8], byte: u8) -> Option<&[u8]> {
        let pos = Self::find(bytes, byte)?;
        // SAFETY: `Search` implementation guarantee that the index is in bounds
        unsafe { Some(bytes.get_unchecked(..pos + 1)) }
    }

    #[inline]
    fn split_byte(bytes: &[u8], byte: u8) -> Option<(&[u8], &[u8])> {
        let pos = Self::find(bytes, byte)?;
        // SAFETY: `Search` implementation guarantee that the index is in bounds
        unsafe { Some(bytes.split_at_unchecked(pos + 1)) }
    }
}

/// Sequential search implementation of [`Search`].
#[derive(Debug)]
pub struct DefaultSearch;

unsafe impl Search for DefaultSearch {
    #[inline]
    fn find(bytes: &[u8], byte: u8) -> Option<usize> {
        bytes.iter().position(|&b| b == byte)
    }
}

// ===== parsers =====

/// Read a CRLF delimited line.
///
/// Advance `reader` to consume the line, and returns the line without its delimiter.
///
/// This function also accepts bare LF delimiter.
#[inline]
pub fn parse_line<'a, S: Search>(reader: &mut Reader<'a>) -> Result<&'a [u8], InsufficientBuffer> {
    let Some(line) = S::find_as_bytes(reader.as_bytes(), b'\n') else {
        return Err(InsufficientBuffer);
    };
    reader.assume_read_len(line.len());
    let suffix = line.last_chunk::<2>().filter(|s| s[0] == b'\r').is_some() as usize;
    Ok(unsafe { line.get_unchecked(..line.len() - (suffix + 1)) })
}
