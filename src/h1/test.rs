use core::mem::MaybeUninit;

use crate::bytes::{Reader, Writer};
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
    let mut buf = [const { MaybeUninit::uninit() }; 32];
    macro_rules! test_me {
        ($reqline:literal; $m:literal, $t:literal, $v:literal;) => {
            let state = RequestLine::parse($reqline).unwrap();
            assert_eq!(state.method, $m);
            assert_eq!(state.target, $t);
            assert_eq!(state.version, $v);
            let ser = state.serialize(&mut buf).unwrap();
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
fn test_status_line() {
    let status = StatusLine::parse(b"HTTP/1.1 200 OK").unwrap();

    assert_eq!(status.version, b"HTTP/1.1");
    assert_eq!(status.status, b"200");
    assert_eq!(status.reason, b"OK");

    let mut buf = [const { MaybeUninit::uninit() }; 32];
    let mut writer = Writer::new(&mut buf);
    writer.write_with(|b| status.serialize(b)).unwrap();

    assert_eq!(writer.init(), b"HTTP/1.1 200 OK\r\n");
}

#[test]
fn test_parse_field() {
    let field = Field::parse(b"Host: example.com").unwrap();
    assert_eq!(field.name, b"Host");
    assert_eq!(field.value, b"example.com");
}

#[test]
fn test_serialize_field() {
    let mut buf = [const { MaybeUninit::uninit() }; 32];
    let mut writer = Writer::new(&mut buf);
    writer
        .write_with(|b| Field { name: b"Host", value: b"example.com" }.serialize(b))
        .unwrap();
    assert_eq!(writer.init(), b"Host: example.com\r\n");
}

#[test]
fn test_parse_request() {
    let message = b"GET / HTTP/1.1\r\nHost:  example.com\r\nContent-Length:127\n\n";
    let mut reader = Reader::new(message);

    let line = reader.read_with(h1::read_line).unwrap();
    let reqline = RequestLine::parse(line).unwrap();

    assert_eq!(reqline.method, b"GET");
    assert_eq!(reqline.target, b"/");
    assert_eq!(reqline.version, b"HTTP/1.1");

    let mut fields = [const { MaybeUninit::uninit() }; 32];
    let mut fields = Fields::new(&mut fields);

    loop {
        let line = reader.read_with(h1::read_line).unwrap();
        if line.is_empty() {
            break;
        }
        fields.parse_field(line).unwrap();
    }

    assert!(!reader.has_remaining());

    let fields = fields.get();
    assert_eq!(fields.len(), 2);

    let expect = [(&b"Host"[..], &b"example.com"[..]), (b"Content-Length", b"127")];

    for (field, expect) in fields.iter().zip(expect) {
        assert_eq!(field.name, expect.0);
        assert_eq!(field.value, expect.1);
    }

    assert!(!reader.has_remaining());
    assert_eq!(message.len(), reader.read_len());
}

#[test]
fn test_serialize_request() {
    let reqline = RequestLine { method: b"GET", target: b"/", version: b"HTTP/1.1" };

    let fields = [Field::new(b"Host", b"example.com"), Field::new(b"Content-Length", b"127")];

    let mut buf = [const { MaybeUninit::uninit() }; 256];
    let mut writer = Writer::new(&mut buf);

    writer.write_with(|b| reqline.serialize(b)).unwrap();
    for field in fields {
        writer.write_with(|b| field.serialize(b)).unwrap();
    }
    writer.write_with(|b| Field::serialize_eoh(b)).unwrap();

    assert_eq!(writer.init(), b"GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 127\r\n\r\n")
}
