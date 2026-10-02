//! [`Slab`] associated types.
use core::ptr::NonNull;
use core::{fmt, hint, mem};

// ===== Slab =====

pub struct Slab<T> {
    entry: NonNull<Entry<T>>,
    len: usize,
    cap: usize,
    last_remove: usize,
}

enum Entry<T> {
    Some(T),
    None(usize),
}

impl<T> Slab<T> {
    /// # Safety
    ///
    /// See [`from_raw_parts_mut`][1].
    ///
    /// Additionally, `ptr` must be aligned of `T`.
    ///
    /// [1]: core::slice::from_raw_parts_mut
    #[inline]
    pub const unsafe fn from_buf(ptr: *mut u8, cap: usize) -> Self {
        Self {
            entry: unsafe { NonNull::new_unchecked(ptr.cast()) },
            len: 0,
            cap: cap / size_of::<T>(),
            last_remove: 0,
        }
    }

    #[inline]
    pub const fn as_ptr(&self) -> *const u8 {
        self.entry.as_ptr().cast()
    }

    #[inline]
    pub const fn peek_key(&self) -> usize {
        self.last_remove
    }

    #[inline]
    pub fn insert(&mut self, value: T) -> Result<usize, InsertError<T>> {
        if self.len == self.cap {
            return Err(InsertError { value });
        }

        let target_ptr = unsafe { self.entry.add(self.last_remove) };
        let key = self.last_remove;

        self.last_remove = if self.last_remove == self.len {
            // appending
            unsafe { target_ptr.write(Entry::Some(value)) };
            self.len += 1;
            self.len
        } else {
            // recycling slot
            let old_entry = unsafe { target_ptr.replace(Entry::Some(value)) };

            let Entry::None(next_delete) = old_entry else {
                unreachable!();
            };
            next_delete
        };

        Ok(key)
    }

    #[inline]
    pub const fn get(&self, key: usize) -> Option<&T> {
        if key < self.len {
            match unsafe { self.entry.add(key).as_ref() } {
                Entry::Some(ok) => Some(ok),
                Entry::None(_) => None,
            }
        } else {
            None
        }
    }

    #[inline]
    pub const fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        if key < self.len {
            match unsafe { self.entry.add(key).as_mut() } {
                Entry::Some(ok) => Some(ok),
                Entry::None(_) => None,
            }
        } else {
            None
        }
    }

    #[inline]
    pub fn remove(&mut self, key: usize) -> Option<T> {
        if key >= self.len {
            return None;
        }
        let target = unsafe { self.entry.add(key).as_mut() };
        if matches!(target, Entry::None(_)) {
            // dangling key
            return None;
        }
        let Entry::Some(ok) = mem::replace(target, Entry::None(self.last_remove)) else {
            unsafe { hint::unreachable_unchecked() }
        };
        self.last_remove = key;
        Some(ok)
    }
}

// ===== Error =====

/// An error that occur if there is not enough capacity when inserting slab entry.
pub struct InsertError<T> {
    value: T,
}

impl<T> InsertError<T> {
    #[inline]
    pub fn into_value(self) -> T {
        self.value
    }
}

impl<T> fmt::Debug for InsertError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("InsertFailed").finish_non_exhaustive()
    }
}

impl<T> fmt::Display for InsertError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to insert slab entry: insufficient capacity")
    }
}
