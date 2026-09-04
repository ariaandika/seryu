use core::mem::MaybeUninit;
use core::{result, slice};

use crate::bytes::{InsufficientBuffer, Writer};
use crate::h1::matches;

// ===== Header =====

/// Raw header name and value.
///
/// Each field line consists of a `case-insensitive` field name followed by a colon, optional
/// leading whitespace, the field line value, and optional trailing whitespace.
///
/// ```not_rust
/// field-line   = field-name ":" OWS field-value OWS
/// ```
#[derive(Debug)]
pub struct Header<'a> {
    pub name: &'a [u8],
    pub value: &'a [u8],
}

impl<'a> Header<'a> {
    /// Creates new [`Header`].
    #[inline]
    pub const fn new(name: &'a [u8], value: &'a [u8]) -> Self {
        Self { name, value }
    }

    /// Parse header from raw bytes.
    #[inline]
    pub const fn parse(bytes: &'a [u8]) -> Result<Self> {
        let mut me = MaybeUninit::uninit();
        match parse_header(bytes, &mut me) {
            // SAFETY: `parse_reqline` guarantee that `Header` initialized
            Ok(()) => Ok(unsafe { me.assume_init() }),
            Err(err) => Err(err),
        }
    }

    /// Returns the required capacity to serialize header.
    #[inline]
    pub const fn serialize_len(&self) -> usize {
        self.name.len() + self.value.len() + b": \r\n".len()
    }

    /// Serialize header to given writer.
    #[inline]
    pub const fn serialize(&self, writer: &mut Writer) -> Result<(), InsufficientBuffer> {
        if writer.remaining() < self.serialize_len() {
            return Err(InsufficientBuffer);
        }
        unsafe {
            writer.write_unchecked(self.name);
            writer.write_unchecked(b": ");
            writer.write_unchecked(self.value);
            writer.write_unchecked(b"\r\n");
        }
        Ok(())
    }

    /// Serialize end of headers (`\r\n`).
    #[inline]
    pub const fn serialize_eoh(writer: &mut Writer) -> Result<(), InsufficientBuffer> {
        writer.write(b"\r\n")
    }
}

/// Parse header from raw bytes.
#[inline]
pub const fn parse_header<'a>(
    header: &'a [u8],
    output: &mut MaybeUninit<Header<'a>>,
) -> Result<()> {
    let Some(name) = matches::find::<b':'>(header) else {
        return Err(HeaderError::InvalidSeparator);
    };
    let off = name.len() + 1;
    let value = unsafe { slice::from_raw_parts(header.as_ptr().add(off), header.len() - off) };
    matches::write_field!(output.name, name);
    matches::write_field!(output.value, value.trim_ascii_start());
    Ok(())
}

// ===== Headers =====

#[derive(Debug)]
pub struct Headers<'a, 'b> {
    buf: &'a mut [MaybeUninit<Header<'b>>],
    len: usize,
}

impl<'a, 'b> Headers<'a, 'b> {
    #[inline]
    pub const fn new(buf: &'a mut [MaybeUninit<Header<'b>>]) -> Self {
        Self { buf, len: 0 }
    }

    #[inline]
    pub const fn get(&self) -> &'a [Header<'b>] {
        unsafe { slice::from_raw_parts(self.buf.as_ptr().sub(self.len).cast(), self.len) }
    }

    #[inline]
    pub const fn remaining(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    pub const fn has_remaining(&self) -> bool {
        self.remaining() != 0
    }

    #[inline]
    pub const fn parse_header(&mut self, bytes: &'b [u8]) -> Result<()> {
        let Some(header_mut) = self.buf.first_mut() else {
            return Err(HeaderError::InsufficientHeaderBuf);
        };
        if let Err(err) = parse_header(bytes, header_mut) {
            return Err(err);
        }
        unsafe {
            self.buf = slice::from_raw_parts_mut(self.buf.as_mut_ptr().add(1), self.buf.len() - 1)
        };
        self.len += 1;
        Ok(())
    }
}

// ===== errors =====

/// `Result` alias for header parsing result.
pub type Result<T, E = HeaderError> = result::Result<T, E>;

/// An error that may occur when parsing headers.
#[derive(Debug)]
pub enum HeaderError {
    /// Given bytes does not contains the end of headers delimiter.
    MissingEndOfHeaders,
    /// Provided header buffer is insufficient.
    InsufficientHeaderBuf,
    /// Invalid header separator.
    InvalidSeparator,
}
