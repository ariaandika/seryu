use core::ffi;
use core::ptr::NonNull;

use genos::mem;

static mut BRK: *mut ffi::c_void = 0 as _;

pub fn init_brk() {
    let brk = mem::brk(0 as _);
    let off = brk.align_offset(8);
    let new_brk = if off != 0 { mem::brk(unsafe { brk.add(off).as_ptr() }) } else { brk };
    unsafe { BRK = new_brk.as_ptr() };
}

pub fn leak_slice<T>(len: usize) -> NonNull<T> {
    let og = unsafe { BRK };

    let align_size = (len * size_of::<T>()).next_multiple_of(8);
    let target = unsafe { og.add(align_size) };
    let ptr = mem::brk(target);

    assert_eq!(ptr.as_ptr(), target, "out of memory");
    unsafe { BRK = target };
    ptr.cast()
}
