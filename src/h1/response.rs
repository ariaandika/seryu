use core::mem::MaybeUninit;

use crate::bytes::{InsufficientBuffer, Writer};
use crate::h1::{ParseError, matches};

const VERSION_SIZE: usize = b"HTTP/1.1".len();
const STATUS_SIZE: usize = 3;
const PREFIX_SIZE: usize = VERSION_SIZE + 2 + STATUS_SIZE;

/// `HTTP/1.1` Message Status Line.
///
/// The first line of a response message is the status-line, consisting of the protocol version, a
/// space, the status code, and another space and ending with an OPTIONAL textual phrase describing
/// the status code.
///
/// ```not_rust
/// status-line = HTTP-version SP status-code SP [ reason-phrase ]
/// ```
///
/// The `status-code` element is a 3-digit integer code describing the result of the server's
/// attempt to understand and satisfy the client's corresponding request.
///
/// ```not_rust
/// status-code    = 3DIGIT
/// ```
///
/// The `reason-phrase` element exists for the sole purpose of providing a textual description
/// associated with the numeric status code, mostly out of deference to earlier Internet application
/// protocols that were more frequently used with interactive text clients.
///
/// ```not_rust
/// reason-phrase  = 1*( HTAB / SP / VCHAR / obs-text )
/// ```
#[derive(Debug, Clone)]
pub struct StatusLine<'a> {
    /// Response version.
    pub version: &'a [u8; VERSION_SIZE],
    /// Response status code.
    pub status: &'a [u8; STATUS_SIZE],
    /// Response status reason phrase.
    pub reason: &'a [u8],
}

impl<'a> StatusLine<'a> {
    /// Parse [`StatusLine`] from raw bytes.
    ///
    /// See the struct documentation for more details on the syntax.
    #[inline]
    pub fn parse(status_line: &'a [u8]) -> Result<Self, ParseError> {
        let mut me = MaybeUninit::uninit();
        parse_status_line(status_line, &mut me)?;
        // SAFETY: `parse_status_line` guarantee that its initialized
        Ok(unsafe { me.assume_init() })
    }

    /// Returns the required capacity to serialize status line.
    #[inline]
    pub const fn serialize_len(&self) -> usize {
        PREFIX_SIZE + self.reason.len() + b"\r\n".len()
    }

    /// Serialize status line with CRLF suffix to the given writer.
    ///
    /// See the struct documentation for more details on the syntax.
    #[inline]
    pub const fn serialize(&self, writer: &mut Writer) -> Result<(), InsufficientBuffer> {
        if writer.remaining() < self.serialize_len() {
            return Err(InsufficientBuffer);
        }
        unsafe {
            writer.write_unchecked(self.version);
            writer.write_unchecked(b" ");
            writer.write_unchecked(self.status);
            writer.write_unchecked(b" ");
            writer.write_unchecked(self.reason);
            writer.write_unchecked(b"\r\n");
        }
        Ok(())
    }
}

/// Parse status line.
///
/// See [`StatusLine`] for more details.
pub fn parse_status_line<'a>(
    line: &'a [u8],
    output: &mut MaybeUninit<StatusLine<'a>>,
) -> Result<(), ParseError> {
    let Some((prefix, rest)) = line.split_first_chunk::<PREFIX_SIZE>() else {
        return Err(ParseError::InsufficientBytes);
    };

    if prefix[VERSION_SIZE] != b' ' {
        return Err(ParseError::InvalidSeparator);
    }
    if prefix[VERSION_SIZE + 1 + STATUS_SIZE] != b' ' {
        return Err(ParseError::InvalidSeparator);
    }

    let (version, status) = split_array::<{ VERSION_SIZE + 1 }, { STATUS_SIZE + 1 }, _>(prefix);

    matches::write_field!(output.version, shrink_array(version));
    matches::write_field!(output.status, shrink_array(status));
    matches::write_field!(output.reason, rest);
    Ok(())
}

const fn split_array<const O1: usize, const O2: usize, const I: usize>(
    arr: &[u8; I],
) -> (&[u8; O1], &[u8; O2]) {
    const { assert!(O1 + O2 == I) };
    // SAFETY: `O1 + O2 == I`
    unsafe { (shrink_array(arr), &*arr.as_ptr().add(O1).cast()) }
}

const fn shrink_array<const I: usize, const O: usize>(arr: &[u8; I]) -> &[u8; O] {
    const { assert!(O < I) };
    // SAFETY: `O < I`
    unsafe { &*arr.as_ptr().cast() }
}
