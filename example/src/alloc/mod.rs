pub use brk::{init_brk, leak_slice};
pub use pool::{MemoryPool, Page};

mod brk;
mod pool;
