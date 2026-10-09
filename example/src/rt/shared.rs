use genos::arch;

use crate::task::FdTable;

pub struct Shared {
    pub fds: FdTable,
}

impl Shared {
    pub fn get<'a>() -> &'a mut Self {
        unsafe { &mut *(arch::rdgsbase() as *mut Self) }
    }

    pub fn setup(&mut self) {
        unsafe { arch::wrgsbase(self as *mut Self as u64) };
    }
}
