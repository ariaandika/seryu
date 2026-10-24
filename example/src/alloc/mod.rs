pub use mmap::{PAGE_SIZE, Result, array, free_array};
pub use pool::{MemoryPool, Page};

mod mmap;
mod pool;
