use core::ffi::CStr;
use core::fmt;

use crate::matches;

/// Postgres row description.
#[derive(Debug, Clone)]
pub struct Row<'a> {
    pub name: &'a CStr,
    pub table_oid: u32,
    pub column_attrno: u16,
    pub dtype_oid: u32,
    pub dtype_size: i16,
    pub format: i16,
}

/// Postgres rows description iterator.
#[derive(Clone)]
pub struct Rows<'a> {
    len: u16,
    body: &'a [u8],
}

impl<'a> Rows<'a> {
    pub(super) fn new(len: u16, body: &'a [u8]) -> Self {
        Self { len, body }
    }
}

impl<'a> Iterator for Rows<'a> {
    type Item = Row<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.len = self.len.checked_sub(1)?;
        let name = matches::find_nul(self.body)?;
        let (name, rest) = unsafe { self.body.split_at_unchecked(name.len() + 1) };
        let name = unsafe { CStr::from_bytes_with_nul_unchecked(name) };
        let (body, rest) = rest.split_first_chunk::<14>()?;
        self.body = rest;
        Some(Row {
            name,
            table_oid: u32::from_be_bytes(*body[..4].as_array()?),
            column_attrno: u16::from_be_bytes(*body[4..6].as_array()?),
            dtype_oid: u32::from_be_bytes(*body[6..10].as_array()?),
            dtype_size: i16::from_be_bytes(*body[10..12].as_array()?),
            format: i16::from_be_bytes(*body[12..14].as_array()?),
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len as usize;
        (len, Some(len))
    }
}

impl<'a> fmt::Debug for Rows<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
