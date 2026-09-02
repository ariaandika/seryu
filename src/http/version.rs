use core::fmt;

/// HTTP Version.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(Inner);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Inner {
    H09,
    H10,
    H11,
    H2,
    H3,
}

impl Version {
    /// `HTTP/0.9`
    pub const HTTP_09: Version = Version(Inner::H09);
    /// `HTTP/1.0`
    pub const HTTP_10: Version = Version(Inner::H10);
    /// `HTTP/1.1`
    pub const HTTP_11: Version = Version(Inner::H11);
    /// `HTTP/2.0`
    pub const HTTP_2: Version = Version(Inner::H2);
    /// `HTTP/3.0`
    pub const HTTP_3: Version = Version(Inner::H3);

    /// Returns string representation of HTTP version.
    ///
    /// The string syntax is defined in [Section 2.3][1] of [HTTP/1.1][2], for example: `"HTTP/1.1"`
    ///
    /// [1]: <https://www.rfc-editor.org/info/rfc9112/#name-http-version>
    /// [2]: <https://www.rfc-editor.org/info/rfc9112/>
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self.0 {
            Inner::H09 => "HTTP/0.9",
            Inner::H10 => "HTTP/1.0",
            Inner::H11 => "HTTP/1.1",
            Inner::H2 => "HTTP/2.0",
            Inner::H3 => "HTTP/3.0",
        }
    }
}

impl Default for Version {
    #[inline]
    fn default() -> Version {
        Version::HTTP_11
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
