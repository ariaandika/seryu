use core::marker;
use core::pin::Pin;

use genos::arch;

use crate::shared::Global;
use crate::task::FdTable;

// ===== impl Global =====

impl Global for FdTable {
    fn global<'a>() -> &'a mut Self {
        &mut Shared::global().fds
    }
}

// ===== Shared =====

pub struct Shared {
    pub fds: FdTable,
    _p: marker::PhantomPinned,
}

impl Shared {
    pub fn new(fds: FdTable) -> Self {
        Self { fds, _p: marker::PhantomPinned }
    }

    pub fn setup(self: Pin<&mut Self>) {
        unsafe { arch::wrgsbase(self.get_unchecked_mut() as *mut Self as u64) };
    }

    fn global<'a>() -> &'a mut Self {
        unsafe { &mut *(arch::rdgsbase() as *mut Self) }
    }
}
