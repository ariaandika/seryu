use std::pin::Pin;
use std::ptr::NonNull;
use std::task::Poll::{self, *};
use std::task::ready;
use std::{cmp, io, mem};
use tcio::bytes::{Bytes, BytesMut};

use crate::body::decoder::Decoder;
use crate::body::error::ReadError;

enum Shared {
    None,
    Read {
        // is read to end is required
        is_to_end: bool,
    },
    Ok {
        data: Bytes,
        // is this the last body chunk
        is_eof: bool,
    },
    Eof,
    Error,
}

pub struct Handle {
    shared: Shared,
    _p: std::marker::PhantomPinned,
}

pub struct HandleRef<'a> {
    shared: NonNull<Shared>,
    _p: std::marker::PhantomData<&'a ()>,
}

unsafe impl Send for Shared {}
unsafe impl Send for Handle {}
unsafe impl Send for HandleRef<'_> {}

impl Handle {
    pub(crate) fn new() -> Self {
        Self {
            shared: Shared::None,
            _p: std::marker::PhantomPinned,
        }
    }

    pub(crate) fn handle_ref(self: Pin<&mut Self>) -> HandleRef<'_> {
        let me = unsafe { self.get_unchecked_mut() };
        HandleRef {
            shared: NonNull::from_mut(&mut me.shared),
            _p: std::marker::PhantomData,
        }
    }

    /// Check for data request from recv handle.
    ///
    /// If decoder returns an error, the user handle will be signaled that an error occur, and this
    /// method will returns `Err`. Note that despite an error occur, user handle must not be
    /// cancelled and keep being polled.
    ///
    /// Returns `Poll::Pending` if more data read is required.
    pub fn poll_read(
        &mut self,
        read_buffer: &mut BytesMut,
        decoder: &mut Decoder,
        mut io: Pin<&mut impl tcio::io::AsyncRead>,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<()>> {
        loop {
            if self.poll_read_inner(&mut *read_buffer, decoder).is_ready() {
                return Ready(Ok(()));
            }

            match ready!(io.as_mut().poll_read(&mut *read_buffer, cx)) {
                Ok(read) => {
                    if read == 0 {
                        self.shared = Shared::Error;
                        return Ready(Err(io::ErrorKind::ConnectionAborted.into()));
                    }

                    // continue
                }
                Err(err) => {
                    self.shared = Shared::Error;
                    return Ready(Err(err));
                }
            }
        }
    }

    fn poll_read_inner(&mut self, read_buffer: &mut BytesMut, decoder: &mut Decoder) -> Poll<()> {
        use Shared as S;

        let S::Read { is_to_end } = self.shared else {
            return Ready(());
        };

        let read = cmp::min(decoder.remaining(), read_buffer.len() as u64);

        if is_to_end && decoder.remaining() != read {
            return Pending;
        }

        let data = unsafe { read_buffer.split_to_unchecked(read as usize).freeze() };

        decoder.advance(read);
        self.shared = S::Ok {
            data,
            is_eof: decoder.is_eof(),
        };

        Ready(())
    }
}

impl HandleRef<'_> {
    pub fn poll_read(&mut self) -> Poll<Option<Result<Bytes, ReadError>>> {
        use Shared as S;

        let shared = unsafe { self.shared.as_mut() };

        match shared {
            S::None => {
                *shared = S::Read { is_to_end: false };
                Pending
            }
            S::Read { .. } => Pending,
            S::Ok { data, is_eof } => {
                let data = mem::take(data);
                *shared = if *is_eof { S::Eof } else { S::None };
                Ready(Some(Ok(data)))
            }
            S::Eof => Ready(None),
            S::Error => Ready(Some(Err(ReadError::new()))),
        }
    }
}

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle").finish_non_exhaustive()
    }
}

impl std::fmt::Debug for HandleRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandleRef").finish_non_exhaustive()
    }
}

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shared::None => write!(f, "None"),
            Shared::Read { .. } => write!(f, "Read"),
            Self::Ok { data, .. } => write!(f, "Ok(..{})", data.len()),
            Self::Eof => write!(f, "Eof"),
            Self::Error => write!(f, "Error"),
        }
    }
}
