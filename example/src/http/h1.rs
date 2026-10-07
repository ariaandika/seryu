use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::task::Poll;

use genos::io;
use genos::net::Socket;
use seryu::bytes::{Reader, Writer};
use seryu::error::IoError;
use seryu::h1;

use crate::Error;
use crate::alloc::Arena;

pub fn handle(state: &mut HttpState, client: &Socket) -> Poll<Result<(), Error>> {
    // ===== Request =====

    let buf = unsafe { state.arena.spare_capacity_mut()[..state.read].assume_init_ref() };
    let mut reader = Reader::new(buf);

    let line = match h1::read_line(&mut reader) {
        Ok(ok) => ok,
        Err(err) => {
            return match err {
                h1::ParseError::InsufficientBytes => Poll::Pending,
                _ => Poll::Ready(Err(err.into())),
            };
        }
    };

    let mut reqline_buf = MaybeUninit::uninit();
    let reqline = h1::parse_reqline(line, &mut reqline_buf)?;

    println!("{reqline:?}");

    let mut fields = h1::Fields::new(unsafe { &mut *state.fields_buf.as_ptr() });

    loop {
        let line = h1::read_line(&mut reader)?;
        if line.is_empty() {
            break;
        }
        fields.parse_field(line)?;
    }

    println!("{:#?}", fields);

    // ===== Response =====

    let mut writer = Writer::new(state.arena.spare_capacity_mut());

    h1::StatusLine { version: b"HTTP/1.1", status: b"200", reason: b"OK" }
        .serialize(&mut writer)?;

    h1::Field::new(b"Content-Length", b"0").serialize(&mut writer)?;
    h1::Field::serialize_eoh(&mut writer)?;

    io::write(client, writer.init())?;

    Poll::Ready(Ok(()))
}

// ===== HttpState =====

type FieldsBuf = [MaybeUninit<h1::Field<'static>>; 32];

pub struct HttpState {
    arena: Arena,
    read: usize,
    fields_buf: NonNull<FieldsBuf>,
}

impl HttpState {
    pub fn new(mut arena: Arena) -> Result<Self, Error> {
        let fields_buf = arena.alloc(size_of::<FieldsBuf>())?.cast();
        Ok(Self { arena, read: 0, fields_buf })
    }

    pub fn on_socket(&mut self, socket: &Socket) -> Poll<()> {
        match self.on_socket_inner(socket) {
            Ok(()) => handle(self, socket).map(drop),
            Err(err) => {
                eprintln!("client error: {err}");
                Poll::Ready(())
            }
        }
    }

    fn on_socket_inner(&mut self, socket: &Socket) -> Result<(), IoError> {
        let buf = &mut self.arena.spare_capacity_mut()[self.read..];
        self.read += io::read(socket, buf)?;
        Ok(())
    }
}

impl From<HttpState> for Arena {
    fn from(value: HttpState) -> Self {
        value.arena
    }
}
