//! Server building blocks.
#![no_std]
#![allow(clippy::module_inception, clippy::new_without_default, clippy::len_without_is_empty)]

pub mod alloc;
pub mod event;
pub mod os;

pub mod bytes;
mod fmt;
mod matches;

pub mod h1;
pub mod http;

pub mod postgres;
