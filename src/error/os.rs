use genos::{arch, sys};

pub type ReadError = sys::Error<arch::sys_read>;

pub type ReadvError = sys::Error<arch::sys_readv>;

pub type WriteError = sys::Error<arch::sys_write>;

pub type WritevError = sys::Error<arch::sys_writev>;
