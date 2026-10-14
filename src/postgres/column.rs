use crate::fmt;

/// Postgres column.
#[derive(Clone)]
pub struct Column<'a> {
    value: &'a [u8],
}

/// Postgres columns iterator.
#[derive(Clone)]
pub struct Columns<'a> {
    len: u16,
    body: &'a [u8],
}

impl<'a> Columns<'a> {
    pub(super) fn new(len: u16, body: &'a [u8]) -> Self {
        Self { len, body }
    }
}

impl<'a> Iterator for Columns<'a> {
    type Item = Column<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.len = self.len.checked_sub(1)?;
        let (len, rest) = self.body.split_first_chunk()?;
        let len = u32::from_be_bytes(*len);
        let (value, rest) = rest.split_at_checked(len as usize)?;
        self.body = rest;
        Some(Column { value })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len as usize;
        (len, Some(len))
    }
}

impl<'a> fmt::Debug for Columns<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a> fmt::Debug for Column<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Column")
            .field(&fmt::debug_ascii(self.value))
            .finish()
    }
}
