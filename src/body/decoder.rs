use std::task::Poll::{self, *};
use tcio::bytes::BytesMut;

// FEAT: chunked encoding in request are not yet supported

pub struct Decoder {
    remaining: u64,
}

impl Decoder {
    pub fn new_length(len: u64) -> Self {
        Self { remaining: len }
    }
}

impl Decoder {
    /// Returns the remaining bytes.
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    pub fn is_eof(&self) -> bool {
        self.remaining == 0
    }

    pub fn advance(&mut self, cnt: u64) {
        self.remaining -= cnt;
    }

    pub fn decode_chunk(&mut self, read_buffer: &mut BytesMut) -> Poll<Option<BytesMut>> {
        if self.remaining == 0 {
            return Ready(None);
        }
        match read_buffer.try_split_to(self.remaining as usize) {
            Some(read_buf) => Ready(Some(read_buf)),
            None => Pending,
        }
    }
}
