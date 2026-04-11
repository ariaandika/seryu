use crate::headers::{HeaderName, HeaderValue};

type Size = u32;

/// Header Field.
///
/// Contains [`HeaderName`] and multiple [`HeaderValue`].
#[derive(Clone)]
pub struct HeaderField {
    hash: Size,
    name: HeaderName,
    value: HeaderValue,
}

impl HeaderField {
    pub(crate) const fn new(name: HeaderName, value: HeaderValue) -> Self {
        Self {
            hash: name.hash(),
            name,
            value,
        }
    }

    pub(crate) const fn with_hash(name: HeaderName, value: HeaderValue, hash: u32) -> Self {
        Self {
            hash,
            name,
            value,
        }
    }

    /// name must be in lowercase
    pub(crate) fn cached_hash(&self) -> Size {
        self.hash
    }

    /// Returns reference to [`HeaderName`].
    #[inline]
    pub const fn name(&self) -> &HeaderName {
        &self.name
    }

    /// Returns reference to [`HeaderValue`].
    #[inline]
    pub const fn value(&self) -> &HeaderValue {
        &self.value
    }

    /// Consume [`HeaderField`] into [`HeaderName`] and [`HeaderValue`].
    ///
    /// Extra header value will be dropped.
    #[inline]
    pub fn into_parts(self) -> (HeaderName, HeaderValue) {
        (self.name, self.value)
    }
}

impl std::fmt::Debug for HeaderField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeaderField")
            .field("name", &self.name)
            .field("value", &self.value)
            .finish()
    }
}
