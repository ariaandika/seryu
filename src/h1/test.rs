use std::mem::MaybeUninit;

use crate::bytes::Reader;
use crate::h1::{self, DefaultSearch, Headers};

#[test]
fn test_message() {
    let message = b"GET / HTTP/1.1\r\nHost:  example.com\r\nContent-Type:127\n\n";
    let mut reader = Reader::new(message);

    let line = h1::parse_line::<DefaultSearch>(&mut reader).unwrap();
    assert_eq!(line, b"GET / HTTP/1.1");

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

    let expect = [(&b"Host"[..], &b"example.com"[..]), (b"Content-Type", b"127")];

    for (header, expect) in headers.iter().zip(expect) {
        assert_eq!(header.name, expect.0);
        assert_eq!(header.value, expect.1);
    }
}
