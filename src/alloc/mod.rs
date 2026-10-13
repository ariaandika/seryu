pub use buf::{Buffer, Capacity};
pub use slab::Slab;
pub use error::{OutOfCapacity, OutOfMemory};

mod error;

pub mod buf;
pub mod slab;
