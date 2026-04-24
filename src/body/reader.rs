use std::task::Poll;
use tcio::bytes::BytesMut;

use crate::body::HandleRef;
use crate::body::error::ReadError;
use crate::body::shared::HandleReadToEnd;

#[derive(Debug)]
pub struct BodyReader<'a> {
    handle: HandleRef<'a>,
}

impl<'a> BodyReader<'a> {
    pub(crate) fn new(handle: HandleRef<'a>) -> Self {
        Self { handle }
    }

    pub fn poll_read(&mut self) -> Poll<Option<Result<BytesMut, ReadError>>> {
        self.handle.poll_read()
    }

    pub fn read_to_end(self) -> ReadToEnd<'a> {
        ReadToEnd {
            handle: self.handle.read_to_end(),
        }
    }
}

#[derive(Debug)]
pub struct ReadToEnd<'a> {
    handle: HandleReadToEnd<'a>,
}

impl Future for ReadToEnd<'_> {
    type Output = Result<BytesMut, ReadError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        unsafe { self.map_unchecked_mut(|e| &mut e.handle).poll(cx) }
    }
}
