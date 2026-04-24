//! HTTP Body Message.

// proto
mod chunked;
mod decoder;

// sync
mod shared;

// public api
mod response;

// error
pub mod error;

// ===== Summary =====

pub(crate) use shared::Handle;

pub use shared::HandleRef;
pub use response::ResponseBody;
