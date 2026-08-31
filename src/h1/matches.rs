use core::slice::from_raw_parts;

#[inline]
pub const fn split_to_delim(bytes: &[u8], delim: u8) -> Option<(&[u8], &[u8])> {
    let mut len = 0;

    loop {
        if len >= bytes.len() {
            return None;
        }
        if unsafe { *bytes.as_ptr().add(len) } == delim {
            break;
        }
        len += 1;
    }

    unsafe { Some(split_at_delim(bytes, len)) }
}

#[inline]
pub const unsafe fn split_at_delim(bytes: &[u8], at: usize) -> (&[u8], &[u8]) {
    let base = bytes.as_ptr();
    let offset = at + 1;
    unsafe { (from_raw_parts(base, at), from_raw_parts(base.add(offset), bytes.len() - offset)) }
}

macro_rules! write_field {
    ($o:ident.$f:ident, $v:expr) => {
        unsafe { (&raw mut (*$o.as_mut_ptr()).$f).write($v) };
    };
}

pub(super) use write_field;
