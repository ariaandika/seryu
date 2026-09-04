//! `HTTP/1.1` Message Syntax ([RFC9112]).
//!
//! This module provide `HTTP/1.1` message parsing and serializing.
//!
//! [RFC9112]: <https://www.rfc-editor.org/info/rfc9112/>
//!
//! # HTTP Message
//!
//! An `HTTP/1.1` message consists of a `start-line` followed by a `CRLF`, zero or more header field
//! lines, an empty line indicating the end of the header section, and an optional message body.
//!
//! A message can be either a request from client to server or a response from server to client.
//!
//! ```not_rust
//! HTTP-message   = start-line CRLF
//!                  *( field-line CRLF )
//!                  CRLF
//!                  [ message-body ]
//! start-line     = request-line / status-line
//! ```
//!
//! A recipient MUST parse an HTTP message as a sequence of octets in an encoding that is a superset
//! of US-ASCII.
//!
//! Although the line terminator for the `start-line` and fields is the sequence `CRLF`, a recipient
//! MAY recognize a single `LF` as a line terminator and ignore any preceding `CR`.
//!
//! In the interest of robustness, a server that is expecting to receive and parse a `request-line`
//! SHOULD ignore at least one empty line (`CRLF`) received prior to the request-line.
//!
//! # Usage
//!
//! Use [`read_line`] to read a `CRLF` delimited line.
//!
//! See [`RequestLine`] and [`StatusLine`] for more details on `request-line` and `status-line`.
//!
//! See [`Field`] for more details on `field-line`.
mod matches;
mod line;
mod request;
mod response;
mod field;
mod error;

pub mod target;

#[cfg(test)]
mod test;

// ===== reexports =====

pub use line::read_line;
pub use request::{RequestLine, parse_reqline};
pub use response::{StatusLine, parse_status_line};
pub use field::{Field, Fields, parse_field};
pub use error::{ParseError, Result};
