use core::fmt;
use core::mem::MaybeUninit;

use genos::io;
use genos::event::poll::Pollfd;
use seryu::bytes::Reader;
use seryu::h1;
use seryu::os::Listener;

fn main() -> Result<(), Error> {
    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;

    Pollfd::new(&listener, Pollfd::IN).poll(-1)?;

    let client = listener.accept()?;

    Pollfd::new(&client, Pollfd::IN).poll(-1)?;

    let mut buf = [const { MaybeUninit::uninit() }; 1024];
    let len = io::read(&client, &mut buf)?;
    let mut reader = Reader::new(unsafe { buf[..len].assume_init_ref() });

    println!("{:?}", str::from_utf8(reader.as_bytes()));

    let line = h1::read_line(&mut reader)?;
    let mut buf = MaybeUninit::uninit();
    let reqline = h1::parse_reqline(line, &mut buf)?;

    println!("{reqline:?}");

    let mut buf = [const { MaybeUninit::uninit() }; 64];
    let mut fields = h1::Fields::new(&mut buf);

    loop {
        let line = h1::read_line(&mut reader)?;
        if line.is_empty() {
            break;
        }
        fields.parse_field(line)?;
    }

    for field in fields.get() {
        println!("{:?} = {:?}", str::from_utf8(field.name), str::from_utf8(field.value));
    }

    Ok(())
}

#[derive(Debug)]
struct Error;

impl<E: fmt::Display> From<E> for Error {
    fn from(value: E) -> Self {
        eprintln!("{value}");
        Self
    }
}
