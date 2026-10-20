use core::fmt;

// ===== Params =====

/// Parameter OIDs.
#[derive(Clone)]
pub struct Params<'a> {
    len: usize,
    params: &'a [u8],
}

impl<'a> Params<'a> {
    pub(super) fn new(len: usize, params: &'a [u8]) -> Self {
        Self { len, params }
    }
}

impl Iterator for Params<'_> {
    type Item = u32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let len = self.len.checked_sub(1)?;
        let (oid, params) = self.params.split_first_chunk::<4>()?;
        *self = Self { len, params };
        Some(u32::from_be_bytes(*oid))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a> fmt::Debug for Params<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
