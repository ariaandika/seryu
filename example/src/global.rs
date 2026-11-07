use core::mem::offset_of;

use seryu::event::Epoll;

pub fn setup_global<T>(ptr: *const T) {
    unsafe { genos::arch::wrgsbase(ptr as u64) };
}

pub struct Global<'a> {
    pub poll: &'a Epoll,
}

pub fn poll<'a>() -> &'a Epoll {
    unsafe { &*(genos::arch::rdgs::<{ offset_of!(Global, poll) }>() as *const _) }
}
