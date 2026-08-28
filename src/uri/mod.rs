//! Uniform Resource Identifier ([RFC3986])
//!
//! [RFC3986]: <https://www.rfc-editor.org/info/rfc3986/>
mod validate;

mod host;
mod authority;
mod target;
mod error;

pub use host::RegName;
pub use authority::Authority;
pub use target::Target;
pub use error::UriError;
