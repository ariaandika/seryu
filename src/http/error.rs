use core::{error, fmt};

// ===== Unknown Method =====

/// An error that occur when parsing unknown method.
#[derive(Debug, Default)]
pub struct UnknownMethod;

impl error::Error for UnknownMethod {}

impl fmt::Display for UnknownMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown method")
    }
}
