//! # Server and Client Toolkit
//!
//! This library provide a toolkit for building a server and client for various different
//! protocols.
//!
//! # Library Design
//!
//! This library is design so that it can be used as building block for writing a server.
//! Additionally, it also provide a ready to use API that combine all components to run a server.
//! It can also be used as an example to use and integrate each available components.
//!
//! # Usage
//!
//! User can use each APIs individually to build custom server, or use available APIs from
//! [`server`] to quickly run a server.
//!
//! [RFC3986]: <https://www.rfc-editor.org/rfc/rfc3986.html>
//! [RFC9110]: <https://www.rfc-editor.org/rfc/rfc9110.html>
//! [RFC9110 Section 5]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-fields>
//! [RFC9110 Section 6]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-message-abstraction>
//! [RFC9112]: <https://www.rfc-editor.org/rfc/rfc9112.html>
//! [RFC9113]: <https://www.rfc-editor.org/rfc/rfc9112.html>
#![warn(missing_debug_implementations)]

mod log;
mod matches;

mod method;
mod status;

mod scheme;
mod authority;
mod target;
mod uri;

pub mod headers;

pub mod body;
pub mod request;
pub mod response;
pub mod error;

pub mod date;

// ===== reexports =====
pub use tcio::bytes;

pub use method::Method;
pub use status::StatusCode;

pub use scheme::Scheme;
pub use authority::Authority;
pub use target::Target;
pub use uri::HttpUri;
