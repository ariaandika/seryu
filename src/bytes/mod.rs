//! Types for working with raw bytes.
pub use read::Reader;
pub use write::Writer;
pub use error::InsufficientBuffer;

mod read;
mod write;
mod error;
