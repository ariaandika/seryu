use core::ptr::NonNull;

use genos::mem;
use seryu::alloc::OutOfMemory;
use seryu::error::IoError;

use crate::alloc::Arena;

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
        let ptr = unsafe { self.base.add(Arena::CAPACITY * idx) };
        Ok(Arena::new(ptr))
    }

    pub fn release(&mut self, arena: Arena) {
        let idx = unsafe { arena.as_ptr().offset_from_unsigned(self.base) } / Arena::CAPACITY;
        self.bitmap |= 1u64 << idx;
    }
}
