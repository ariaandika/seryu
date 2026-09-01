use core::{error, fmt};

/// An error that occur when buffer capacity is insufficient to perform the operation.
#[derive(Debug)]
pub struct InsufficientBuffer;

impl error::Error for InsufficientBuffer {}

impl fmt::Display for InsufficientBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("insufficient buffer")
    }
}
