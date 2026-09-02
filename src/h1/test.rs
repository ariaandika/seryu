use core::mem::MaybeUninit;

use crate::bytes::{Reader, Writer};
use crate::h1::target::Origin;
use crate::h1::{self, DefaultSearch, Header, Headers, RequestLine, StatusLine};

#[test]
fn test_parse_line() {
    let line = b"Host: example.com\r\n";
    let mut reader = Reader::new(line);
    let parsed = h1::parse_line::<DefaultSearch>(&mut reader).unwrap();
    assert!(!reader.has_remaining());
    assert_eq!(parsed, b"Host: example.com");

    let line = b"Host: example.com\n";
    let mut reader = Reader::new(line);
    let parsed = h1::parse_line::<DefaultSearch>(&mut reader).unwrap();
    assert!(!reader.has_remaining());
    assert_eq!(parsed, b"Host: example.com");

    let line = b"\n";
    let mut reader = Reader::new(line);
    let parsed = h1::parse_line::<DefaultSearch>(&mut reader).unwrap();
    assert!(!reader.has_remaining());
    assert_eq!(parsed, b"");
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

#[test]
fn test_parse_origin() {
    let target = b"/users?id=42";
    let origin = Origin::parse(target).unwrap();
    assert_eq!(origin.path, b"/users");
    assert_eq!(origin.query, Some(&b"?id=42"[..]));
}

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

#[test]
fn test_parse_field() {
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
fn test_parse_request() {
    let message = b"GET / HTTP/1.1\r\nHost:  example.com\r\nContent-Length:127\n\n";
    let mut reader = Reader::new(message);

    let line = h1::parse_line::<DefaultSearch>(&mut reader).unwrap();
    let reqline = RequestLine::parse(line).unwrap();

    assert_eq!(reqline.method, b"GET");
    assert_eq!(reqline.target, b"/");
    assert_eq!(reqline.version, b"HTTP/1.1");

    let mut headers = [const { MaybeUninit::uninit() }; 32];
    let mut headers = Headers::new(&mut headers);

    loop {
        let line = h1::parse_line::<DefaultSearch>(&mut reader).unwrap();
        if line.is_empty() {
            break;
        }
        headers.parse_header::<DefaultSearch>(line).unwrap();
    }

    assert!(!reader.has_remaining());

    let headers = headers.get();
    assert_eq!(headers.len(), 2);

    let expect = [(&b"Host"[..], &b"example.com"[..]), (b"Content-Length", b"127")];

    for (header, expect) in headers.iter().zip(expect) {
        assert_eq!(header.name, expect.0);
        assert_eq!(header.value, expect.1);
    }

    assert!(!reader.has_remaining());
    assert_eq!(message.len(), reader.read_len());
}

#[test]
fn test_serialize_request() {
    let reqline = RequestLine { method: b"GET", target: b"/", version: b"HTTP/1.1" };

    let headers =
        [h1::Header::new(b"Host", b"example.com"), h1::Header::new(b"Content-Length", b"127")];

    let mut buf = [const { MaybeUninit::uninit() }; 256];
    let mut writer = Writer::new(&mut buf);

    reqline.serialize(&mut writer).unwrap();
    for header in headers {
        header.serialize(&mut writer).unwrap();
    }
    h1::Header::serialize_eoh(&mut writer).unwrap();

    assert_eq!(writer.init(), b"GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 127\r\n\r\n")
}
