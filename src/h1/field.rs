use core::mem::MaybeUninit;
use core::slice;

use crate::bytes::{InsufficientBuffer, Writer};
use crate::h1::error::{ParseError, Result};
use crate::h1::matches;

// ===== Field =====

/// `HTTP/1.1` Message Field Line.
///
/// Field line consists of a `case-insensitive` field name followed by a colon, optional
/// leading whitespace, the field line value, and optional trailing whitespace.
///
/// ```not_rust
/// field-line   = field-name ":" OWS field-value OWS
/// ```
#[derive(Debug)]
pub struct Field<'a> {
    /// Field name.
    pub name: &'a [u8],
    /// Field value.
    pub value: &'a [u8],
}

impl<'a> Field<'a> {
    /// Creates new [`Field`].
    #[inline]
    pub const fn new(name: &'a [u8], value: &'a [u8]) -> Self {
        Self { name, value }
    }

    /// Parse [`Field`] from raw bytes.
    #[inline]
    pub const fn parse(bytes: &'a [u8]) -> Result<Self> {
        let mut field = MaybeUninit::uninit();
        match parse_field(bytes, &mut field) {
            // SAFETY: `parse_reqline` guarantee that its initialized
            Ok(()) => Ok(unsafe { field.assume_init() }),
            Err(err) => Err(err),
        }
    }

    /// Returns the required capacity to serialize field.
    #[inline]
    pub const fn serialize_len(&self) -> usize {
        self.name.len() + self.value.len() + b": \r\n".len()
    }

    /// Serialize field to given writer.
    #[inline]
    pub const fn serialize(&self, writer: &mut Writer) -> Result<(), InsufficientBuffer> {
        if writer.remaining() < self.serialize_len() {
            return Err(InsufficientBuffer);
        }
        unsafe {
            writer.write_unchecked(self.name);
            writer.write_unchecked(b": ");
            writer.write_unchecked(self.value);
            writer.write_unchecked(b"\r\n");
        }
        Ok(())
    }

    /// Serialize end of header fields (`\r\n`).
    #[inline]
    pub const fn serialize_eoh(writer: &mut Writer) -> Result<(), InsufficientBuffer> {
        writer.write(b"\r\n")
    }
}

/// Parse field from raw bytes.
#[inline]
pub const fn parse_field<'a>(bytes: &'a [u8], output: &mut MaybeUninit<Field<'a>>) -> Result<()> {
    let Some(name) = matches::find::<b':'>(bytes) else {
        return Err(ParseError::InvalidSeparator);
    };
    let off = name.len() + 1;
    let value = unsafe { slice::from_raw_parts(bytes.as_ptr().add(off), bytes.len() - off) };
    matches::write_field!(output.name, name);
    matches::write_field!(output.value, value.trim_ascii_start());
    Ok(())
}

// ===== Fields =====

/// [`Field`] slice container.
#[derive(Debug)]
pub struct Fields<'a, 'b> {
    buf: &'a mut [MaybeUninit<Field<'b>>],
    len: usize,
}

impl<'a, 'b> Fields<'a, 'b> {
    /// Create new [`Fields`] with given buffer capacity.
    #[inline]
    pub const fn new(buf: &'a mut [MaybeUninit<Field<'b>>]) -> Self {
        Self { buf, len: 0 }
    }

    /// Returns the initialized fields.
    #[inline]
    pub const fn get(&self) -> &'a [Field<'b>] {
        unsafe { slice::from_raw_parts(self.buf.as_ptr().sub(self.len).cast(), self.len) }
    }

    /// Returns the remaining buffer capacity.
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.buf.len()
    }

    /// Returns `true` if buffer has remaining capacity left.
    #[inline]
    pub const fn has_remaining(&self) -> bool {
        self.remaining() != 0
    }

    /// Parse field from raw bytes and store it in the buffer.
    ///
    /// # Errors
    ///
    /// Returns error if parsing failed or there is no remaining buffer capacity left.
    #[inline]
    pub const fn parse_field(&mut self, bytes: &'b [u8]) -> Result<()> {
        let Some(field_mut) = self.buf.first_mut() else {
            return Err(ParseError::InsufficientBuf);
        };
        if let Err(err) = parse_field(bytes, field_mut) {
            return Err(err);
        }
        unsafe {
            self.buf = slice::from_raw_parts_mut(self.buf.as_mut_ptr().add(1), self.buf.len() - 1)
        };
        self.len += 1;
        Ok(())
    }
}
