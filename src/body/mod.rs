//! HTTP Body Message.

// proto
mod chunked;
mod decoder;

// sync
mod shared;

// public api
mod reader;
mod response;

// error
pub mod error;

// ===== Summary =====

pub(crate) use shared::{Handle, HandleRef};

pub use reader::{BodyReader, ReadToEnd};
pub use response::ResponseBody;
