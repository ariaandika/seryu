use core::mem::MaybeUninit;

use seryu::bytes::{Reader, Writer};
use seryu::h1;

use crate::Error;

pub type HeadersBuf<'a> = [MaybeUninit<h1::Field<'a>>; MAX_HEADERS as usize];

const MAX_HEADERS: u32 = 24;
const MAX_HEADERS_SIZE: usize = 4096;

pub fn request_handler(reqline: &h1::RequestLine, headers: &[h1::Field]) {
    println!("{reqline:?}");
    for header in headers {
        println!("{header:?}");
    }
}

pub fn response_writer(writer: &mut Writer) -> Result<(), Error> {
    writer.write_with(|b| {
        h1::StatusLine { version: b"HTTP/1.1", status: b"200", reason: b"OK" }.serialize(b)
    })?;
    writer.write_with(|b| h1::Field::new(b"Content-Length", b"0").serialize(b))?;
    writer.write_with(|b| h1::Field::serialize_eoh(b))?;
    Ok(())
}

/// Returns the length of the request in bytes.
///
/// Returns `0` if the there is not enough bytes to complete parsing.
pub fn scan_request(bytes: &[u8]) -> Result<u32, Error> {
    let mut reader = Reader::new(bytes);
    for _ in 0..=MAX_HEADERS {
        if reader.read_len() > MAX_HEADERS_SIZE {
            break;
        }
        match reader.read_with(h1::read_line) {
            Some(line) => {
                if line.is_empty() {
                    return Ok(reader.read_len() as _);
                }
            }
            None => return Ok(0),
        }
    }
    Err("excessive headers".into())
}

pub fn parse_request<'a>(
    bytes: &'a [u8],
    reqline: &mut MaybeUninit<h1::RequestLine<'a>>,
    headers: &mut HeadersBuf<'a>,
) -> Result<u32, Error> {
    let mut reader = Reader::new(bytes);
    let Some(line) = reader.read_with(h1::read_line) else {
        return Err("insufficient bytes".into());
    };
    h1::parse_reqline(line, reqline)?;
    let mut header_len = 0;
    while let Some(header_mut) = headers.get_mut(header_len as usize) {
        let Some(line) = reader.read_with(h1::read_line) else {
            return Err("insufficient bytes".into());
        };
        if line.is_empty() {
            break;
        }
        h1::parse_field(line, header_mut)?;
        header_len += 1;
    }
    Ok(header_len)
}
