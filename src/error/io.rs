use core::fmt;

use genos::sys;

#[derive(Debug)]
pub struct IoError {
    name: Option<&'static str>,
    code: sys::ErrCode,
}

impl IoError {
    /// Returns `true` if error code is `EAGAIN`.
    pub const fn is_retry(&self) -> bool {
        self.code.is_retry()
    }
}

impl<S: sys::SysId> From<sys::Error<S>> for IoError {
    #[inline]
    fn from(value: sys::Error<S>) -> Self {
        Self { name: Some(value.syscall_name()), code: value.code() }
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name {
            write!(f, "`{name}` returns ")?;
        }
        write!(f, "{:?}", self.code)
    }
}
