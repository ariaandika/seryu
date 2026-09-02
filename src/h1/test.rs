use core::mem::MaybeUninit;

use crate::bytes::{Reader, Writer};
use crate::h1::{self, DefaultSearch, Headers, RequestLine};

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
