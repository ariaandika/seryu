use core::fmt;
use core::str::FromStr;

use crate::http::UnknownMethod;

/// HTTP Method.
///
/// This API implements methods defined in [RFC9110] and the [PATCH] method.
///
/// [RFC9110]: <https://www.rfc-editor.org/info/rfc9110#name-method-definitions>
/// [PATCH]: <https://www.rfc-editor.org/info/rfc5789>
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Method(Inner);

props! {
    /// The [GET] method.
    ///
    /// [GET]: <https://www.rfc-editor.org/info/rfc9110#name-get>
    pub const GET = Get { safe, idempotent };
    /// The [HEAD] method.
    ///
    /// [HEAD]: <https://www.rfc-editor.org/info/rfc9110#name-head>
    pub const HEAD = Head { safe, idempotent };
    /// The [POST] method.
    ///
    /// [POST]: <https://www.rfc-editor.org/info/rfc9110.html#name-post>
    pub const POST = Post { };
    /// The [PUT] method.
    ///
    /// [PUT]: <https://www.rfc-editor.org/info/rfc9110.html#name-put>
    pub const PUT = Put { idempotent };
    /// The [DELETE] method.
    ///
    /// [DELETE]: <https://www.rfc-editor.org/info/rfc9110.html#name-delete>
    pub const DELETE = Delete { idempotent };
    /// The [CONNECT] method.
    ///
    /// [CONNECT]: <https://www.rfc-editor.org/info/rfc9110#name-connect>
    pub const CONNECT = Connect { };
    /// The [OPTIONS] method.
    ///
    /// [OPTIONS]: <https://www.rfc-editor.org/info/rfc9110#name-options>
    pub const OPTIONS = Options { safe, idempotent };
    /// The [TRACE] method.
    ///
    /// [TRACE]: <https://www.rfc-editor.org/info/rfc9110#name-trace>
    pub const TRACE = Trace { safe, idempotent };
    /// The [PATCH] method.
    ///
    /// [PATCH]: <https://www.rfc-editor.org/info/rfc5789#section-2>
    pub const PATCH = Patch { };
}

// ===== std traits =====

impl FromStr for Method {
    type Err = UnknownMethod;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bytes(s.as_bytes())
    }
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ===== Macros =====

macro_rules! props {
    (
        $(
           $(#[$doc:meta])*
           $vis:vis const $name:ident = $method:ident { $($prop:ident),* };
        )*
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        enum Inner {
            $($method),*
        }

        impl Default for Method {
            #[inline]
            fn default() -> Self {
                Self::GET
            }
        }

        impl Method {
            $(
               $(#[$doc])*
               $vis const $name: Self = Self(Inner::$method);
            )*

            /// Creates `Method` from raw bytes.
            ///
            /// This method only accept uppercase alphabetic.
            #[inline]
            pub const fn from_bytes(bytes: &[u8]) -> Result<Self, UnknownMethod> {
                // there is no `bytestify` to allow placing it in pattern matching
                $(const $name: &[u8] = stringify!($name).as_bytes();)*
                match bytes {
                    $($name => Ok(Self::$name),)*
                    _ => Err(UnknownMethod)
                }
            }

            /// Returns string representation of this method.
            #[inline]
            pub const fn as_str(&self) -> &'static str {
                match &self.0 {
                    $(Inner::$method => stringify!($name),)*
                }
            }

            /// Returns `true` if method is considered ["safe"].
            ///
            /// ["safe"]: <https://www.rfc-editor.org/info/rfc9110.html#name-safe-methods>
            #[inline]
            pub const fn is_safe(&self) -> bool {
                match &self.0 {
                    $(Inner::$method => safe!($($prop),*),)*
                }
            }

            /// Returns `true` if method is considered ["idempotent"].
            ///
            /// ["idempotent"]: <https://www.rfc-editor.org/info/rfc9110.html#name-idempotent-methods>
            #[inline]
            pub const fn is_idempoten(&self) -> bool {
                match &self.0 {
                    $(Inner::$method => idempotent!($($prop),*),)*
                }
            }
        }
    };
}

macro_rules! safe {
    (safe, $($tt:ident),*) => { true };
    (safe) => { true };
    ($tt:ident, $($t2:ident)*) => { safe!($($t2)*) };
    ($tt:ident) => { false };
    () => { false };
}
macro_rules! idempotent {
    (idempotent, $($tt:ident),*) => { true };
    (idempotent) => { true };
    ($tt:ident, $($t2:ident)*) => { idempotent!($($t2)*) };
    ($tt:ident) => { false };
    () => { false };
}
use idempotent;
use props;
use safe;
