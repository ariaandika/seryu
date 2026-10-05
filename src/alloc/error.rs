use core::fmt;

#[derive(Debug)]
pub struct OutOfMemory;

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "out of memory")
    }
}
