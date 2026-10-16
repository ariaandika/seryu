use core::mem::MaybeUninit;
use core::ptr;

use crate::bytes::BufMut;
use crate::h1::target::Origin;
use crate::h1::{self, Field, Fields, RequestLine, StatusLine};

#[test]
fn test_read_line() {
    macro_rules! test_me {
        ($b:literal, $line:literal) => {
            test_me!($b, $line, b"");
        };
        ($b:literal, $line:literal, $rest:literal) => {
            let mut bytes: &[u8] = $b;
            assert_eq!(h1::read_line(&mut bytes).unwrap(), $line);
            assert_eq!(bytes, $rest);
        };
    }

    test_me!(b"Host: example.com\r\n", b"Host: example.com");
    test_me!(b"H\r\n", b"H");
    test_me!(b"\r\n", b"");

    test_me!(b"Host: example.com\n", b"Host: example.com");
    test_me!(b"H\n", b"H");
    test_me!(b"\n", b"");

    test_me! {
        b"Host\x80: example.com\n",
        b"Host", b": example.com\n"
    };
}

#[test]
fn test_reqline() {
    macro_rules! test_me {
        ($reqline:literal; $m:literal, $t:literal, $v:literal;) => {
            let state = RequestLine::parse($reqline).unwrap();
            assert_eq!(state.method, $m);
            assert_eq!(state.target, $t);
            assert_eq!(state.version, $v);

            let mut writer = Writer::<32>::new();
            state.serialize(&mut writer).unwrap();
            let (reqline, crlf) = writer.init().split_at(writer.len() - 2);
            assert_eq!(reqline, $reqline);
            assert_eq!(crlf, b"\r\n");
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

#[test]
fn test_status_line() {
    let status = StatusLine::parse(b"HTTP/1.1 200 OK").unwrap();
    assert_eq!(status.version, b"HTTP/1.1");
    assert_eq!(status.status, b"200");
    assert_eq!(status.reason, b"OK");

    let mut writer = Writer::<32>::new();
    status.serialize(&mut writer).unwrap();
    assert_eq!(writer.init(), b"HTTP/1.1 200 OK\r\n");
}

#[test]
fn test_field() {
    let field = Field::parse(b"Host: example.com").unwrap();
    assert_eq!(field.name, b"Host");
    assert_eq!(field.value, b"example.com");

    let mut writer = Writer::<32>::new();
    field.serialize(&mut writer).unwrap();
    assert_eq!(writer.init(), b"Host: example.com\r\n");
}

#[test]
fn test_parse_origin() {
    macro_rules! t {
        ($t:literal, ($p:literal, $q:expr)) => {
            let origin = Origin::parse($t).unwrap();
            assert_eq!(origin.path, $p);
            assert_eq!(origin.query, $q);
        };
    }
    t!(b"/users", (b"/users", None));
    t!(b"/users?", (b"/users", Some(&b"?"[..])));
    t!(b"/users?id=42", (b"/users", Some(&b"?id=42"[..])));
}

#[test]
fn test_parse_request() {
    let message = b"GET / HTTP/1.1\r\nHost:  example.com\r\nContent-Length:127\n\n";
    let mut reader = &message[..];

    let line = h1::read_line(&mut reader).unwrap();
    let reqline = RequestLine::parse(line).unwrap();

    assert_eq!(reqline.method, b"GET");
    assert_eq!(reqline.target, b"/");
    assert_eq!(reqline.version, b"HTTP/1.1");

    let mut fields = [const { MaybeUninit::uninit() }; 32];
    let mut fields = Fields::new(&mut fields);

    loop {
        let line = h1::read_line(&mut reader).unwrap();
        if line.is_empty() {
            break;
        }
        fields.parse_field(line).unwrap();
    }

    assert!(reader.is_empty());

    let fields = fields.get();
    assert_eq!(fields.len(), 2);

    let expect = [(&b"Host"[..], &b"example.com"[..]), (b"Content-Length", b"127")];

    for (field, expect) in fields.iter().zip(expect) {
        assert_eq!(field.name, expect.0);
        assert_eq!(field.value, expect.1);
    }

    assert!(reader.is_empty());
}

#[test]
fn test_serialize_request() {
    let reqline = RequestLine { method: b"GET", target: b"/", version: b"HTTP/1.1" };
    let fields = [Field::new(b"Host", b"example.com"), Field::new(b"Content-Length", b"127")];

    let mut writer = Writer::<256>::new();

    reqline.serialize(&mut writer).unwrap();
    for field in fields {
        field.serialize(&mut writer).unwrap();
    }
    Field::serialize_eoh(&mut writer).unwrap();

    assert_eq!(writer.init(), b"GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 127\r\n\r\n")
}

// ===== Helper =====

struct Writer<const N: usize> {
    buf: [u8; N],
    write: usize,
}

impl<const N: usize> Writer<N> {
    fn new() -> Self {
        Self { buf: [0; N], write: 0 }
    }

    fn init(&self) -> &[u8] {
        &self.buf[..self.write]
    }

    fn len(&self) -> usize {
        self.write
    }
}

impl<const N: usize> BufMut for &mut Writer<N> {
    fn spare_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe {
            let ptr = &raw mut self.buf[self.write];
            &mut *ptr::slice_from_raw_parts_mut(ptr.cast(), N)
        }
    }

    unsafe fn advance_mut(&mut self, count: usize) {
        debug_assert!(self.write + count <= N);
        self.write += count;
    }
}
