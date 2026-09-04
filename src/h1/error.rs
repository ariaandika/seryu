use core::{error, fmt, result};

/// `Result` alias for `HTTP/1.1` message parsing result.
pub type Result<T, E = ParseError> = result::Result<T, E>;

/// An error that may occur when parsing `HTTP/1.1` message.
#[derive(Debug)]
pub enum ParseError {
    /// Given bytes is not enough to complete parsing.
    InsufficientBytes,
    /// Buffer has no remaining capacity left.
    InsufficientBuf,
    /// Invalid or missing separator.
    InvalidSeparator,
}

impl ParseError {
    const fn message(&self) -> &'static str {
        match self {
            Self::InsufficientBytes => "insufficient bytes",
            Self::InsufficientBuf => "insufficient buffer",
            Self::InvalidSeparator => "invalid separator",
        }
    }
}

impl error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message().fmt(f)
    }
}
