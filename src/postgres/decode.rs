use core::ffi::CStr;
use core::{error, fmt};

use crate::postgres::SizedRead;

// ===== Decode =====

/// Decodable postgres message.
pub trait Decode<'a>: fmt::Debug + Sized {
    /// Message type.
    const MSGTYPE: u8;

    /// Decode a raw message body.
    fn decode(body: &'a [u8]) -> Result<Self, DecodeError>;
}

// ===== Decoder =====

pub struct Decoder<'a>(&'a [u8]);

impl<'a> Decoder<'a> {
    pub fn new(body: &'a [u8]) -> Self {
        Self(body)
    }

    pub fn once<P: SizedRead<N>, const N: usize>(body: &[u8]) -> Result<P, DecodeError> {
        match body.try_into() {
            Ok(ok) => Ok(P::from_bytes(ok)),
            Err(_) => Err(DecodeError::Insufficient),
        }
    }

    pub fn cstr(body: &[u8]) -> Result<&CStr, DecodeError> {
        match CStr::from_bytes_with_nul(body) {
            Ok(ok) => Ok(ok),
            Err(_) => Err(DecodeError::NoNulTerm),
        }
    }

    pub fn decode_sized<P: SizedRead<N>, const N: usize>(&mut self) -> Result<P, DecodeError> {
        match self.0.split_first_chunk() {
            Some((ok, rest)) => {
                self.0 = rest;
                Ok(P::from_bytes(ok))
            }
            None => Err(DecodeError::Insufficient),
        }
    }

    pub fn remaining(&self) -> &'a [u8] {
        self.0
    }
}

// ===== Error =====

/// An error that may occur when decoding postgres message.
#[derive(Debug)]
pub enum DecodeError {
    /// Malformed message.
    ///
    /// Either unknown message type or length is less than message header size.
    Malformed,
    /// Unexpected message.
    Unexpected,
    /// Bytes is not enough to complete decoding.
    Insufficient,
    /// String is not nul terminated.
    NoNulTerm,
    /// Invalid message semantic.
    ///
    /// The semantic detail are message specific.
    Invalid,
}

impl error::Error for DecodeError {}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => write!(f, "malformed message"),
            Self::Unexpected => write!(f, "unexpected message"),
            Self::Insufficient => write!(f, "not enough bytes"),
            Self::NoNulTerm => write!(f, "string was not nul-terminated"),
            Self::Invalid => write!(f, "invalid message semantic"),
        }
    }
}
