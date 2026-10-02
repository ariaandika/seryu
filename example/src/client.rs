use core::mem::MaybeUninit;

use genos::event::poll::Pollfd;
use genos::io;
use genos::net::Socket;
use seryu::bytes::{Reader, Writer};
use seryu::h1;

use crate::Error;

pub fn handle(client: &Socket) -> Result<(), Error> {
    Pollfd::new(client, Pollfd::IN).poll(-1)?;

    // Request

    let mut buf = [const { MaybeUninit::uninit() }; 1024];
    let len = io::read(client, &mut buf)?;
    let mut reader = Reader::new(unsafe { buf[..len].assume_init_ref() });

    let line = h1::read_line(&mut reader)?;
    let mut reqline_buf = MaybeUninit::uninit();
    let reqline = h1::parse_reqline(line, &mut reqline_buf)?;

    println!("{reqline:?}");

    let mut fields_buf = [const { MaybeUninit::uninit() }; 64];
    let mut fields = h1::Fields::new(&mut fields_buf);

    loop {
        let line = h1::read_line(&mut reader)?;
        if line.is_empty() {
            break;
        }
        fields.parse_field(line)?;
    }

    println!("{:#?}", fields);

    // Response

    let mut writer = Writer::new(&mut buf);

    h1::StatusLine { version: b"HTTP/1.1", status: b"200", reason: b"OK" }
        .serialize(&mut writer)?;

    h1::Field::new(b"Content-Length", b"0").serialize(&mut writer)?;
    h1::Field::serialize_eoh(&mut writer)?;

    io::write(client, writer.init())?;

    Ok(())
}
