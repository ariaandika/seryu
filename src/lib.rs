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
#![no_std]
#![allow(clippy::module_inception, clippy::new_without_default, clippy::len_without_is_empty)]

mod matches;

pub mod bytes;
pub mod http;

pub mod h1;
