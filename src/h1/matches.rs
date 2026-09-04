use core::slice::from_raw_parts;

const BLOCK: usize = size_of::<usize>();
const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);

/// Returns line from the start until, excluding, the delimiter.
pub const fn find<const B: u8>(mut bytes: &[u8]) -> Option<&[u8]> {
    const { assert!(B < 128) };

    let base = bytes.as_ptr();
    let byte = const { usize::from_ne_bytes([B; BLOCK]) };

    while let Some((chunk, rest)) = bytes.split_first_chunk::<BLOCK>() {
        let block = usize::from_ne_bytes(*chunk);
        let result = (block ^ byte).wrapping_sub(LSB) & MSB;
        if result != 0 {
            unsafe {
                let nth = (result.trailing_zeros() / 8) as usize;
                let end_ptr = bytes.as_ptr().add(nth);
                let len = end_ptr.offset_from_unsigned(base);
                return Some(from_raw_parts(base, len));
            }
        }
        bytes = rest;
    }

    loop {
        let [byte, rest @ ..] = bytes else {
            return None;
        };
        if *byte == B {
            unsafe {
                let end_ptr = bytes.as_ptr();
                let len = end_ptr.offset_from_unsigned(base);
                return Some(from_raw_parts(base, len));
            }
        }
        bytes = rest;
    }
}

macro_rules! write_field {
    ($o:ident.$f:ident, $v:expr) => {{
        // dont put it in unsafe context
        let val = $v;
        unsafe { (&raw mut (*$o.as_mut_ptr()).$f).write(val) };
    }};
}

pub(super) use write_field;
