use core::mem::MaybeUninit;
use core::{error, fmt};

use crate::bytes::{InsufficientBuffer, Writer};

/// Request line raw bytes components.
#[derive(Debug, Default, Clone)]
pub struct RequestLine<'a> {
    pub method: &'a [u8],
    pub target: &'a [u8],
    pub version: &'a [u8],
}

impl<'a> RequestLine<'a> {
    #[inline]
    pub fn parse(reqline: &'a [u8]) -> Result<Self, ReqlineError> {
        let mut me = MaybeUninit::uninit();
        parse_reqline(reqline, &mut me)?;
        // SAFETY: `parse_reqline` guarantee that its initialized
        Ok(unsafe { me.assume_init() })
    }

    /// Returns the required capacity to serialize request line.
    #[inline]
    pub const fn serialize_len(&self) -> usize {
        self.method.len() + self.target.len() + self.version.len() + b"  \r\n".len()
    }

    /// Serialize request line to given writer.
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

pub fn parse_reqline<'a, 'b>(
    reqline: &'b [u8],
    output: &'a mut MaybeUninit<RequestLine<'b>>,
) -> Result<&'a mut RequestLine<'b>, ReqlineError> {
    const VERSION_SIZE: usize = b" HTTP/1.1".len();

    let Some(remaining) = reqline.len().checked_sub(VERSION_SIZE) else {
        return Err(ReqlineError::Insufficient);
    };

    // SAFETY: `remaining < reqline.len()`
    let (rest, version) = unsafe { reqline.split_at_unchecked(remaining) };

    // SAFETY: `VERSION_SIZE` contains the leading whitespace
    unsafe { (&raw mut (*output.as_mut_ptr()).version).write(version.get_unchecked(1..)) };

    let mut method_len = 0;
    loop {
        let Some(byte) = rest.get(method_len) else {
            return Err(ReqlineError::InvalidSeparator);
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

// ===== errors =====

#[derive(Debug)]
pub enum ReqlineError {
    Insufficient,
    InvalidSeparator,
}

impl ReqlineError {
    const fn message(&self) -> &'static str {
        match self {
            Self::Insufficient => "insufficient bytes",
            Self::InvalidSeparator => "invalid separator",
        }
    }
}

impl error::Error for ReqlineError {}

impl fmt::Display for ReqlineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message().fmt(f)
    }
}

// ===== tests =====

#[test]
fn test_reqline() {
    let mut buf = [const { MaybeUninit::uninit() }; 32];
    macro_rules! test_me {
        ($reqline:literal; $m:literal, $t:literal, $v:literal;) => {
            let state = RequestLine::parse($reqline).unwrap();
            assert_eq!(state.method, $m);
            assert_eq!(state.target, $t);
            assert_eq!(state.version, $v);
            let mut writer = Writer::new(&mut buf);
            state.serialize(&mut writer).unwrap();
            let ser = writer.init();
            assert_eq!(&ser[..ser.len() - 2], $reqline);
        };
    }
    test_me! {
        b"GET / HTTP/1.1";
        b"GET", b"/", b"HTTP/1.1";
    }
    test_me! {
        b"POST /users HTTP/1.1";
        b"POST", b"/users", b"HTTP/1.1";
    }
    test_me! {
        b"PUT ?id=4040 HTTP/1.1";
        b"PUT", b"?id=4040", b"HTTP/1.1";
    }
    test_me! {
        b"  HTTP/1.1";
        b"", b"", b"HTTP/1.1";
    }
    test_me! {
        b"PUT  HTTP/1.1";
        b"PUT", b"", b"HTTP/1.1";
    }
}
