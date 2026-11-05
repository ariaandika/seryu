//! Task manager.
pub use waker::waker;
pub use task::{TaskId, Interest};

mod waker;
mod task;
