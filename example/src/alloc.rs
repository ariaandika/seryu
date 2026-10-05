use core::ffi;
use core::ptr::NonNull;
use std::mem::MaybeUninit;
use std::slice;

use genos::mem;
use seryu::alloc::OutOfMemory;
use seryu::error::IoError;

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

// ===== MemoryPool =====

type Bitmap = u64;

const REGION: usize = 4096;
const POOL_CAP: usize = 32 * REGION;

pub struct MemoryPool {
    base: NonNull<u8>,
    bitmap: Bitmap,
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        let _ = mem::munmap(self.base.as_ptr().cast(), POOL_CAP);
    }
}

impl MemoryPool {
    pub fn new() -> Result<Self, IoError> {
        use mem::mmap::{self, Flags, Prot};
        let prot = Prot::READ | Prot::WRITE;
        let flags = Flags::PRIVATE | Flags::ANON;
        let base = mmap::mmap_anon(0 as _, POOL_CAP, prot, flags)?.cast();
        Ok(Self { bitmap: Bitmap::MAX, base })
    }

    pub fn create_arena(&mut self) -> Result<Arena, OutOfMemory> {
        // 0 = used, 1 = free
        let bitpos = self.bitmap.isolate_lowest_one();
        if bitpos == 0 {
            return Err(OutOfMemory);
        }
        self.bitmap &= !(bitpos << 1);
        let idx = bitpos.trailing_zeros() as usize;
        let ptr = unsafe { self.base.add(ARENA_CAP * idx) };
        Ok(Arena::new(ptr))
    }

    pub fn release(&mut self, arena: Arena) {
        let idx = unsafe { arena.ptr.offset_from_unsigned(self.base) } / ARENA_CAP;
        self.bitmap |= 1u64 << idx;
    }
}

// ===== Memory =====

const ARENA_CAP: usize = 4096;

pub struct Arena {
    ptr: NonNull<u8>,
    len: usize,
}

impl Arena {
    fn new(ptr: NonNull<u8>) -> Self {
        Self { ptr, len: 0 }
    }

    pub fn alloc(&mut self, len: usize) -> Result<NonNull<u8>, OutOfMemory> {
        let ptr = unsafe { self.ptr.add(self.len) };
        self.len += align8(len);
        if self.len <= ARENA_CAP {
            Ok(ptr)
        } else {
            self.len -= len;
            Err(OutOfMemory)
        }
    }

    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe {
            let ptr = self.ptr.add(self.len).as_ptr().cast();
            slice::from_raw_parts_mut(ptr, ARENA_CAP - self.len)
        }
    }
}

const fn align8(value: usize) -> usize {
    (value + 7) & !7
}
