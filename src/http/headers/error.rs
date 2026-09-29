//! Error types that can occur during header related operation.
use core::{error, fmt};

/// An error that can occur in header related operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderError {
    /// Bytes is empty.
    Empty,
    /// Excessive bytes length.
    ExcessiveBytes,
    /// Bytes contains invalid character.
    Invalid,
}

impl HeaderError {
    pub(crate) const fn invalid_len(len: usize) -> Self {
        match len {
            0 => Self::Empty,
            _ => Self::ExcessiveBytes,
        }
    }

    pub(crate) const fn message(&self) -> &'static str {
        match self {
            Self::Empty => "empty header",
            Self::ExcessiveBytes => "excessive bytes",
            Self::Invalid => "invalid byte",
        }
    }
}

impl error::Error for HeaderError {}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}
