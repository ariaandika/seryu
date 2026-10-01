//! Server building blocks.
#![no_std]
#![allow(clippy::module_inception, clippy::new_without_default, clippy::len_without_is_empty)]

pub mod os;

pub mod alloc;
pub mod bytes;
pub mod error;
mod fmt;
mod matches;

pub mod h1;
pub mod http;
