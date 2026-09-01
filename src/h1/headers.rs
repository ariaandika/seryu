use core::mem::{self, MaybeUninit};
use core::{result, slice};

use crate::bytes::{InsufficientBuffer, Writer};
use crate::h1::line::{DefaultSearch, Search};
use crate::h1::matches;

// ===== Header =====

/// Raw header name and value.
///
/// Note that this does not guarantee for valid header name or value characters.
#[derive(Debug)]
pub struct Header<'a> {
    pub name: &'a [u8],
    pub value: &'a [u8],
}

impl<'a> Header<'a> {
    /// Parse header from raw bytes.
    #[inline]
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let mut me = MaybeUninit::uninit();
        parse_header::<DefaultSearch>(bytes, &mut me)?;
        // SAFETY: `parse_reqline` guarantee that its initialized
        Ok(unsafe { me.assume_init() })
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
}

/// Parse header from raw bytes.
#[inline]
pub const fn parse_header<'a, S: Search>(
    header: &'a [u8],
    output: &mut MaybeUninit<Header<'a>>,
) -> Result<()> {
    let Some((name, val)) = matches::split_to_delim(header, b':') else {
        return Err(HeaderError::InvalidSeparator);
    };
    matches::write_field!(output.name, name);
    matches::write_field!(output.value, val.trim_ascii_start());
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
    pub fn parse_header<S: Search>(&mut self, bytes: &'b [u8]) -> Result<()> {
        let Some((header, rest)) = mem::take(&mut self.buf).split_first_mut() else {
            return Err(HeaderError::InsufficientHeaderBuf);
        };
        parse_header::<S>(bytes, header)?;
        self.buf = rest;
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

// ===== tests =====

#[test]
fn test_parse_header() {
    let header = Header::parse(b"Host: example.com").unwrap();
    assert_eq!(header.name, b"Host");
    assert_eq!(header.value, b"example.com");
}

#[test]
fn test_serialize_header() {
    let mut buf = [const { MaybeUninit::uninit() }; 32];
    let mut writer = Writer::new(&mut buf);
    let header = Header { name: b"Host", value: b"example.com" };
    header.serialize(&mut writer).unwrap();
    assert_eq!(writer.init(), b"Host: example.com\r\n");
}
