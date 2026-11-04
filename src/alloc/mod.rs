pub use alloc::Allocator;
pub use brk::Brk;
pub use mmap::{Mmap, PAGE_SIZE, Result};
pub use error::{OutOfCapacity, OutOfMemory};
pub use arena::Arena;
pub use slab::Slab;

mod alloc;
mod brk;
mod mmap;

mod error;
mod arena;

pub mod slab;
