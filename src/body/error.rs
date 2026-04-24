use std::io;

// ===== Body Error =====

/// A semantic error when reading message body.
#[derive(Debug)]
pub enum BodyError {
    /// Invalid or duplicate Content-Length value.
    InvalidContentLength,
    /// Invalid message body codings.
    InvalidCodings,
    /// Unknown or unsupported `Transfer-Encoding` codings.
    UnknownCodings,
    /// User error where it tries to read empty or exhausted body.
    Exhausted,
    /// User error where body size hint implementation does not match with the chunk length.
    InvalidSizeHint,
    /// Client error where chunked format is invalid.
    InvalidChunked,
    /// Client error where excessive chunk length is received.
    ExcessiveChunk,
}

impl BodyError {
    const fn message(&self) -> &'static str {
        match self {
            Self::InvalidContentLength => "invalid content length",
            Self::InvalidCodings => "invalid message body codings",
            Self::UnknownCodings => "unknown or unsupported message body codings",
            Self::Exhausted => "message body exhausted",
            Self::InvalidSizeHint => "invalid size hint",
            Self::InvalidChunked => "invalid chunked format",
            Self::ExcessiveChunk => "excessive chunk",
        }
    }
}

impl std::error::Error for BodyError { }

impl std::fmt::Display for BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

// ===== Read Body Error =====

/// An error that can occur during body reading.
pub struct ReadError {
    _p: (),
}

impl ReadError {
    pub(crate) fn new() -> Self {
        Self { _p: () }
    }
}

impl std::error::Error for ReadError { }

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "body read error")
    }
}

impl std::fmt::Debug for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ReadError").finish()
    }
}

