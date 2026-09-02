use core::mem::MaybeUninit;

use crate::bytes::{InsufficientBuffer, Writer};
use crate::h1::{ReqlineError, matches};

const VERSION_SIZE: usize = b"HTTP/1.1".len();
const STATUS_SIZE: usize = 3;

const PREFIX_SIZE: usize = VERSION_SIZE + 2 + STATUS_SIZE;

/// Status line raw bytes components.
#[derive(Debug, Clone)]
pub struct StatusLine<'a> {
    pub version: &'a [u8; VERSION_SIZE],
    pub status: &'a [u8; STATUS_SIZE],
    pub reason: &'a [u8],
}

impl<'a> StatusLine<'a> {
    #[inline]
    pub fn parse(status_line: &'a [u8]) -> Result<Self, ReqlineError> {
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

    /// Serialize status line to given writer.
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

pub fn parse_status_line<'a>(
    line: &'a [u8],
    output: &mut MaybeUninit<StatusLine<'a>>,
) -> Result<(), ReqlineError> {
    let Some((prefix, rest)) = line.split_first_chunk::<PREFIX_SIZE>() else {
        return Err(ReqlineError::Insufficient);
    };

    if prefix[VERSION_SIZE] != b' ' {
        return Err(ReqlineError::InvalidSeparator);
    }
    if prefix[VERSION_SIZE + 1 + STATUS_SIZE] != b' ' {
        return Err(ReqlineError::InvalidSeparator);
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

// ===== tests =====

#[test]
fn test_status_line() {
    let state = StatusLine::parse(b"HTTP/1.1 200 OK").unwrap();

    assert_eq!(state.version, b"HTTP/1.1");
    assert_eq!(state.status, b"200");
    assert_eq!(state.reason, b"OK");

    let mut buf = [const { MaybeUninit::uninit() }; 32];
    let mut writer = Writer::new(&mut buf);
    state.serialize(&mut writer).unwrap();

    assert_eq!(writer.init(), b"HTTP/1.1 200 OK\r\n");
}
