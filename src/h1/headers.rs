use core::mem::MaybeUninit;
use core::{result, slice};

use crate::bytes::{InsufficientBuffer, Reader, Writer};
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

/// Parse headers from raw bytes.
pub fn parse_headers<'a, 'h, S: Search>(
    bytes: &mut Reader<'a>,
    buf: &'h mut [MaybeUninit<Header<'a>>],
) -> Result<&'h mut [Header<'a>]> {
    let mut n = 0;
    loop {
        let Some(line) = S::find_as_bytes(bytes.as_bytes(), b'\n') else {
            return Err(HeaderError::MissingEndOfHeaders);
        };

        let header = line.trim_ascii_end();
        if header.is_empty() {
            bytes.assume_read_len(line.len());
            break;
        }

        let Some(output) = buf.get_mut(n) else {
            return Err(HeaderError::InsufficientHeaderBuf);
        };
        output.write(Header::parse(header)?);

        n += 1;
        bytes.assume_read_len(line.len());
    }
    // SAFETY: `n` tracks the initialized headers
    Ok(unsafe { slice::from_raw_parts_mut(buf.as_mut_ptr().cast(), n) })
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

#[test]
fn test_parse_headers() {
    let mut headers = [const { MaybeUninit::uninit() }; 32];
    let bytes = concat!("Host: example.com\r\n", "Content-Length: 472\r\n", "\r\n",).as_bytes();

    let mut reader = Reader::new(bytes);
    let headers = parse_headers::<DefaultSearch>(&mut reader, &mut headers).unwrap();

    assert!(!reader.has_remaining());
    assert_eq!(headers.len(), 2);
    assert_eq!(headers[0].name, b"Host");
    assert_eq!(headers[0].value, b"example.com");
    assert_eq!(headers[1].name, b"Content-Length");
    assert_eq!(headers[1].value, b"472");
}
