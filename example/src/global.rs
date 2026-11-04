use std::mem::offset_of;

use crate::event::Polling;

pub fn setup_global<T>(ptr: *const T) {
    unsafe { genos::arch::wrgsbase(ptr as u64) };
}

pub struct Global<'a> {
    pub poll: &'a Polling,
}

pub fn poll<'a>() -> &'a Polling {
    unsafe { &*(genos::arch::rdgs::<{ offset_of!(Global, poll) }>() as *const _) }
}
