use crate::headers::HeaderMap;
use crate::{Authority, Method, Scheme, Target};

// ===== RequestHead =====

#[derive(Debug)]
pub struct RequestHead {
    pub(crate) method: Method,
    pub(crate) scheme: Scheme,
    pub(crate) authority: Authority,
    pub(crate) target: Target,
    pub(crate) headers: HeaderMap,
}

impl RequestHead {
    /// Returns the request method.
    #[inline]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns `true` if current scheme is https.
    #[inline]
    pub const fn is_https(&self) -> bool {
        self.scheme.is_https()
    }

    /// Returns the request authority.
    ///
    /// In `HTTP/1.1`, this is the value of the `Host` header.
    ///
    /// In `HTTP/2.0`, this is the value of the `:authority` pseudo-header.
    #[inline]
    pub const fn authority(&self) -> &str {
        self.authority.as_str()
    }

    /// Returns the request hostname.
    #[inline]
    pub const fn hostname(&self) -> &str {
        self.authority.host()
    }

    /// Returns string to the request host's port.
    #[inline]
    pub const fn port(&self) -> Option<&str> {
        self.authority.port()
    }

    /// Returns string to the request target path.
    #[inline]
    pub const fn path(&self) -> &str {
        self.target.path()
    }

    /// Returns string to the request target query.
    #[inline]
    pub const fn query(&self) -> Option<&str> {
        self.target.query()
    }

    /// Returns string to the request target path and query.
    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.target.as_str()
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

