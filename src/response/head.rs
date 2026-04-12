use crate::headers::HeaderMap;
use crate::StatusCode;

// ===== ResponseHead =====

#[derive(Debug)]
pub struct ResponseHead {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
}

impl ResponseHead {
    pub fn new(status: StatusCode, headers: HeaderMap) -> Self {
        Self {
            status,
            headers,
        }
    }

    /// Returns the response status code.
    #[inline]
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns shared reference to [`HeaderMap`].
    #[inline]
    pub const fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns mutable reference to [`HeaderMap`].
    #[inline]
    pub const fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }
}
