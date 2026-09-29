//! HTTP Header.
pub use name::{HeaderName, lookup, standard};
pub use value::HeaderValue;

mod matches;
mod name;
mod value;

pub mod error;
