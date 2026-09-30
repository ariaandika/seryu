use core::fmt;
use core::mem::MaybeUninit;

use genos::io;
use seryu::os::Listener;

fn main() -> Result<(), Error> {
    let listener = Listener::bind_tcp([127, 0, 0, 1], 4040)?;

    let client = listener.accept()?;

    let mut buf = [const { MaybeUninit::uninit() }; 1024];
    let len = io::read(&client, &mut buf)?;

    println!("{:?}", str::from_utf8(unsafe { buf[..len].assume_init_ref() }));

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
