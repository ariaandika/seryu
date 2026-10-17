use core::fmt;

use crate::alloc::OutOfCapacity;
use crate::bytes::BufMut;
use crate::postgres::primitive::Primitive;

// ===== Encode =====

/// Encodable postgres message.
pub trait Encode: fmt::Debug {
    /// Message type.
    const MSGTYPE: u8;

    /// Size of the main body.
    fn size(&self) -> usize;

    /// Encode the message body.
    ///
    /// # Safety
    ///
    /// `ptr` must be valid for write up to [`size`][Encode::size] bytes.
    unsafe fn encode_raw(&self, ptr: *mut u8);

    /// Encode full message.
    ///
    /// `buf` must be able to contains at least `size + 5`, where `size` is the return value of
    /// [`Encode::size`].
    #[inline]
    fn encode_msg<B: BufMut>(&self, mut buf: B) -> Result<(), OutOfCapacity> {
        const MSGTYPE_SIZE: usize = size_of::<u8>();
        const LENGTH_SIZE: usize = size_of::<u32>();
        const PREFIX: usize = MSGTYPE_SIZE + LENGTH_SIZE;

        let size = self.size();
        let total_size = PREFIX + size;

        let Some(spare) = buf.spare_mut().get_mut(..total_size) else {
            return Err(OutOfCapacity);
        };
        let mut enc = Encoder::new(spare.as_mut_ptr());

        unsafe {
            buf.advance_mut(total_size);
            enc.encode(Self::MSGTYPE);
            enc.encode((LENGTH_SIZE + size) as u32);
            self.encode_raw(enc.as_mut_ptr());
            Ok(())
        }
    }
}

// ===== Encoder =====

pub struct Encoder(*mut u8);

impl Encoder {
    pub fn new<T>(ptr: *mut T) -> Self {
        Self(ptr.cast())
    }

    pub unsafe fn once<P: Primitive>(ptr: *mut u8, val: P) {
        unsafe { P::write(val, ptr) };
    }

    pub unsafe fn encode<P: Primitive>(&mut self, val: P) {
        unsafe {
            P::write(val, self.0);
            self.0 = self.0.add(val.size());
        }
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0
    }
}
