use core::{error, fmt};

// ===== Out Of Memory =====

/// An error that occur when a syscalls failed to request a memory.
#[derive(Debug)]
pub struct OutOfMemory;

impl error::Error for OutOfMemory {}

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "out of memory")
    }
}

// ===== Out Of Capacity =====

/// An error that occur when remaining buffer capacity is not enough to perform an operation.
#[derive(Debug)]
pub struct OutOfCapacity;

impl error::Error for OutOfCapacity {}

impl fmt::Display for OutOfCapacity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not enough remaining capacity")
    }
}
