use core::ptr::NonNull;

use genos::mem;
use seryu::alloc::OutOfMemory;
use seryu::error::IoError;

// ===== Page =====

const PAGE_SIZE: usize = 4096;

/// A pointer wrapper that is guarantee to have [`PAGE_SIZE`] bytes of valid memory.
#[repr(transparent)]
pub struct Page<const N: usize = 1>(NonNull<[u8; PAGE_SIZE]>);

impl Page {
    pub const SIZE: usize = PAGE_SIZE;
}

impl<const N: usize> Page<N> {
    pub const fn as_ptr(&self) -> NonNull<u8> {
        self.0.cast()
    }
}

// ===== MemoryPool =====

type Bitmap = u64;

const PAGE_SCALE: usize = 4;
const PAGE_CHUNK: usize = PAGE_SIZE * PAGE_SCALE;

const POOL_CAP: usize = 8;
const POOL_SIZE: usize = POOL_CAP * PAGE_CHUNK;

pub struct MemoryPool {
    base: NonNull<u8>,
    bitmap: Bitmap,
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        let _ = mem::munmap(self.base.as_ptr().cast(), POOL_SIZE);
    }
}

impl MemoryPool {
    pub fn new() -> Result<Self, IoError> {
        use mem::mmap::{self, Flags, Prot};
        let prot = Prot::READ | Prot::WRITE;
        let flags = Flags::PRIVATE | Flags::ANON;
        let base = mmap::mmap_anon(0 as _, POOL_SIZE, prot, flags)?.cast();
        let bitmap = !(Bitmap::MAX << (POOL_CAP / PAGE_SCALE));
        Ok(Self { bitmap, base })
    }

    pub fn alloc_page(&mut self) -> Result<Page<PAGE_SCALE>, OutOfMemory> {
        // 0 = used, 1 = free
        let bitpos = self.bitmap.isolate_lowest_one();
        if bitpos == 0 {
            return Err(OutOfMemory);
        }
        self.bitmap &= !bitpos;
        let idx = bitpos.trailing_zeros() as usize;
        Ok(Page(unsafe { self.base.add(PAGE_CHUNK * idx).cast() }))
    }

    pub fn release<const N: usize>(&mut self, page: Page<N>) {
        let idx = unsafe { page.as_ptr().offset_from_unsigned(self.base) } / PAGE_CHUNK;
        self.bitmap |= 1 << idx;
    }
}
