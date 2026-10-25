use core::{mem, ops, ptr};

use genos::event::epoll;

use crate::alloc;

type Buf = [mem::MaybeUninit<epoll::Event>; CAP];

const CAP: usize = alloc::PAGE_SIZE / size_of::<epoll::Event>();

// ===== EventBuf =====

pub struct EventBuf(ptr::NonNull<Buf>);

impl Drop for EventBuf {
    fn drop(&mut self) {
        alloc::free_array(self.0);
    }
}

impl EventBuf {
    pub fn new() -> alloc::Result<Self> {
        alloc::array().map(Self)
    }
}

impl ops::Deref for EventBuf {
    type Target = Buf;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.cast().as_ref() }
    }
}

impl ops::DerefMut for EventBuf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.cast().as_mut() }
    }
}
