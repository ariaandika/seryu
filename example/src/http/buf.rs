use core::ops;
use core::ptr::NonNull;

use seryu::alloc::{Buffer, Capacity};

use crate::alloc::Page;

pub type TaskPage = Page<4>;

// ===== ReqBuffer =====

pub struct IoBuffer(Buffer<u8, IoBufferCap>);

impl IoBuffer {
    /// # Safety
    ///
    /// `ptr` must be valid for read and write up to [`CAPACITY`][1] bytes.
    ///
    /// [1]: IoBufferCap::CAPACITY
    pub unsafe fn from_raw(ptr: NonNull<u8>) -> Self {
        Self(unsafe { Buffer::from_raw_parts(ptr.cast(), 0, IoBufferCap) })
    }
}

impl ops::Deref for IoBuffer {
    type Target = Buffer<u8, IoBufferCap>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for IoBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// ===== impl Capacity =====

pub struct IoBufferCap;

impl IoBufferCap {
    pub const CAPACITY: usize = Page::SIZE;
}

impl Capacity for IoBufferCap {
    fn capacity(&self) -> usize {
        Self::CAPACITY
    }
}
