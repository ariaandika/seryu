use core::mem::MaybeUninit;

use crate::bytes::{InsufficientBuffer, Writer};
use crate::h1::ParseError;

/// `HTTP/1.1` Message Request line.
///
/// A `request-line` begins with a method token, followed by a single space, the `request-target`,
/// and another single space, and ends with the protocol version.
///
/// ```not_rust
/// request-line   = method SP request-target SP HTTP-version
/// ```
///
/// See [`Method`] and [`Version`] for more details on `method` and `version`.
///
/// See [`target`] for more details on `request-target`.
///
/// [`Method`]: crate::http::Method
/// [`Version`]: crate::http::Version
/// [`target`]: crate::h1::target
#[derive(Debug, Default, Clone)]
pub struct RequestLine<'a> {
    /// Request method.
    pub method: &'a [u8],
    /// Request target.
    pub target: &'a [u8],
    /// Request version.
    pub version: &'a [u8],
}

impl<'a> RequestLine<'a> {
    /// Parse [`RequestLine`] from raw bytes.
    ///
    /// See the struct documentation for more details on the syntax.
    #[inline]
    pub fn parse(reqline: &'a [u8]) -> Result<Self, ParseError> {
        let mut me = MaybeUninit::uninit();
        parse_reqline(reqline, &mut me)?;
        // SAFETY: `parse_reqline` guarantee that its initialized
        Ok(unsafe { me.assume_init() })
    }

    /// Returns the required capacity to serialize request line.
    #[inline]
    pub const fn serialize_len(&self) -> usize {
        self.method.len() + self.target.len() + self.version.len() + 4 /*SP SP CR LF*/
    }

    /// Serialize request line with CRLF suffix to the given writer.
    ///
    /// See the struct documentation for more details on the syntax.
    #[inline]
    pub const fn serialize(&self, writer: &mut Writer) -> Result<(), InsufficientBuffer> {
        if writer.remaining() < self.serialize_len() {
            return Err(InsufficientBuffer);
        }
        unsafe {
            writer.write_unchecked(self.method);
            writer.write_unchecked(b" ");
            writer.write_unchecked(self.target);
            writer.write_unchecked(b" ");
            writer.write_unchecked(self.version);
            writer.write_unchecked(b"\r\n");
        }
        Ok(())
    }
}

/// Parse request line.
///
/// See [`RequestLine`] for more details.
pub fn parse_reqline<'a, 'b>(
    reqline: &'b [u8],
    output: &'a mut MaybeUninit<RequestLine<'b>>,
) -> Result<&'a mut RequestLine<'b>, ParseError> {
    const VERSION_SIZE: usize = b" HTTP/1.1".len();

    let Some(remaining) = reqline.len().checked_sub(VERSION_SIZE) else {
        return Err(ParseError::InsufficientBytes);
    };

    // SAFETY: `remaining < reqline.len()`
    let (rest, version) = unsafe { reqline.split_at_unchecked(remaining) };

    // SAFETY: `VERSION_SIZE` contains the leading whitespace
    unsafe { (&raw mut (*output.as_mut_ptr()).version).write(version.get_unchecked(1..)) };

    let mut method_len = 0;
    loop {
        let Some(byte) = rest.get(method_len) else {
            return Err(ParseError::InvalidSeparator);
        };
        if *byte == b' ' {
            break;
        }
        method_len += 1;
    }

    unsafe {
        // SAFETY: `reqline.get(method_len)` returns `Some` guarantee that it is in bounds
        let (method, rest) = rest.split_at_unchecked(method_len);
        let out = output.as_mut_ptr();

        (&raw mut (*out).method).write(method);
        // SAFETY: `rest` contains the leading whitespace
        (&raw mut (*out).target).write(rest.get_unchecked(1..));

        Ok(output.assume_init_mut())
    }
}
