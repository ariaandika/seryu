use core::ffi::CStr;

// ===== SizedRead =====

pub trait SizedRead<const N: usize> {
    fn from_bytes(bytes: &[u8; N]) -> Self;
}

impl SizedRead<1> for u8 {
    fn from_bytes(bytes: &[u8; 1]) -> Self {
        bytes[0]
    }
}

impl SizedRead<2> for u16 {
    fn from_bytes(bytes: &[u8; 2]) -> Self {
        u16::from_be_bytes(*bytes)
    }
}

impl SizedRead<4> for u32 {
    fn from_bytes(bytes: &[u8; 4]) -> Self {
        u32::from_be_bytes(*bytes)
    }
}

// ===== Primitive =====

pub trait Primitive: Copy {
    fn size(&self) -> usize {
        size_of::<Self>()
    }

    unsafe fn write(self, ptr: *mut u8);
}

impl Primitive for u8 {
    unsafe fn write(self, ptr: *mut u8) {
        unsafe { ptr.write(self) };
    }
}

impl Primitive for u16 {
    unsafe fn write(self, ptr: *mut u8) {
        unsafe { ptr.copy_from_nonoverlapping(self.to_be_bytes().as_ptr(), 2) };
    }
}

impl Primitive for u32 {
    unsafe fn write(self, ptr: *mut u8) {
        unsafe { ptr.copy_from_nonoverlapping(self.to_be_bytes().as_ptr(), 4) };
    }
}

impl Primitive for &str {
    fn size(&self) -> usize {
        self.len() + 1
    }

    unsafe fn write(self, ptr: *mut u8) {
        unsafe {
            ptr.copy_from_nonoverlapping(self.as_ptr(), self.len());
            ptr.add(self.len()).write(0);
        }
    }
}

impl Primitive for &CStr {
    fn size(&self) -> usize {
        self.to_bytes_with_nul().len()
    }

    unsafe fn write(self, ptr: *mut u8) {
        let me = self.to_bytes_with_nul();
        unsafe { ptr.copy_from_nonoverlapping(me.as_ptr(), me.len()) };
    }
}

impl Primitive for &[u8] {
    fn size(&self) -> usize {
        self.len()
    }

    unsafe fn write(self, ptr: *mut u8) {
        unsafe { ptr.copy_from_nonoverlapping(self.as_ptr(), self.len()) };
    }
}
