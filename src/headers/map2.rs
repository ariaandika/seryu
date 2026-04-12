//! Second version of the header map API.
use std::num::NonZeroU32;
use std::{mem, ptr, slice};

use crate::headers::error::TryReserveError;
use crate::headers::{HeaderField, HeaderName, HeaderValue};

// space-time tradeoff
// most of integer type is limited
// this limit practically should never exceeded for header length
type Size = u32;

/// HTTP Headers Multimap.
///
/// # Hash Function
///
/// `HeaderMap` **DOES NOT** use hashing algorithm that provide resistance against HashDoS attacks.
/// It is expected that user will limit the number of headers to much lower number than the amount
/// of where HashDoS attack is a concern.
///
/// # Capacity Limitations
///
/// This implementation has a maximum capacity that is lower than the system limit. The exact limit
/// is sufficient for all HTTP headers use cases.
pub struct HeaderMap {
    fields: ptr::NonNull<HeaderField>,
    len: Size,
    cap: Size,
}

type HashIdx = Option<HashField>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HashField {
    hash: u32,
    idx: NonZeroU32,
}

impl HashField {
    fn checked_field<'a>(&self, probe: &Probe, name: &HeaderName) -> Option<&'a HeaderField> {
        if self.hash != probe.hash {
            return None;
        }
        let field = self.field(probe);
        if field.name() == name {
            Some(field)
        } else {
            None
        }
    }

    fn checked_field_mut<'a>(&self, probe: &Probe, name: &HeaderName) -> Option<&'a mut HeaderField> {
        if self.hash != probe.hash {
            return None;
        }
        let field = self.field_mut(probe);
        if field.name() == name {
            Some(field)
        } else {
            None
        }
    }

    fn field_ptr(&self, probe: &Probe) -> ptr::NonNull<HeaderField> {
        // SAFETY: current index, with offset applies, points to the correct field
        unsafe { probe.ptr.cast().add(self.idx.get() as usize) }
    }

    fn field<'a>(&self, probe: &Probe) -> &'a HeaderField {
        unsafe { self.field_ptr(probe).as_ref() }
    }

    fn field_mut<'a>(&self, probe: &Probe) -> &'a mut HeaderField {
        unsafe { self.field_ptr(probe).as_mut() }
    }

    /// SAFETY: `ptr.add(idx)` must be valid for write
    unsafe fn put_field(&self, probe: &Probe, field: HeaderField) {
        unsafe { self.field_ptr(probe).write(field); }
    }
}

unsafe impl Send for HeaderMap {}
unsafe impl Sync for HeaderMap {}

impl Drop for HeaderMap {
    fn drop(&mut self) {
        // dangling ptr
        if self.cap == 0 {
            return;
        }
        // call drop on fields except the hash table
        unsafe { ptr::drop_in_place(self.fields_mut()) };
        // deallocate
        alloc::deallocate(self.fields, self.cap);
    }
}

impl Clone for HeaderMap {
    fn clone(&self) -> Self {
        if self.is_empty() {
            return Self::new();
        }

        // allocate without initializing the hash table
        let fields = alloc::allocate(self.cap);
        let offset = alloc::offset(self.cap);

        // copy the hash table
        unsafe { fields.copy_from_nonoverlapping(self.fields, offset as usize) };

        // clone the fields
        for i in offset..offset + self.len {
            unsafe {
                let dst = fields.add(i as usize).as_mut();
                let src = self.fields.add(i as usize).as_ref();
                dst.clone_from(src);
            }
        }

        Self {
            fields,
            len: self.len,
            cap: self.cap,
        }
    }
}

impl Default for HeaderMap {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl HeaderMap {
    /// Create new empty [`HeaderMap`].
    ///
    /// This function does not allocate.
    #[inline]
    pub const fn new() -> Self {
        Self {
            fields: ptr::NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }

    /// Creates new empty [`HeaderMap`] with at least the specified capacity.
    ///
    /// The header map will be able to hold at least capacity headers without reallocating. If
    /// capacity is zero, the header map will not allocate.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the capacity limit.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity).expect("failed to create HeaderMap")
    }

    /// Creates new empty [`HeaderMap`] with at least the specified capacity.
    ///
    /// The header map will be able to hold at least capacity headers without reallocating. If
    /// capacity is zero, the header map will not allocate.
    ///
    /// # Errors
    ///
    /// Returns an error if the new capacity exceeds the capacity limit.
    #[inline]
    pub fn try_with_capacity(capacity: usize) -> Result<Self, TryReserveError> {
        if capacity == 0 {
            return Ok(Self::new());
        }
        match Size::try_from(capacity) {
            Ok(cap) => Ok(Self::with_capacity_size(cap)),
            Err(_) => Err(TryReserveError {}),
        }
    }

    /// Creates new empty [`HeaderMap`] with at least the specified capacity.
    #[inline]
    fn with_capacity_size(cap: Size) -> Self {
        Self {
            fields: alloc::allocate_init(cap),
            len: 0,
            cap,
        }
    }

    /// Returns headers length.
    ///
    /// This length includes duplicate header name.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len as _
    }

    /// Returns the total number of elements the map can hold without reallocating.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.cap as _
    }

    /// Returns `true` if headers has no element.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns an iterator over header fields.
    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, HeaderField> {
        self.fields().iter()
    }

    /// Returns an iterator over headers as name and value pair.
    #[inline]
    pub fn pairs(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.fields().iter().map(|f|(f.name(), f.value()))
    }

    /// Returns `true` if the map contains a header value for given header name.
    #[inline]
    pub fn contains_key(&self, name: &HeaderName) -> bool {
        if self.is_empty() {
            return false
        }
        Probe::new(self, name.hash()).get(name).is_some()
    }

    /// Returns a reference to the first header value corresponding to the given header name.
    ///
    /// ```rust
    /// use seryu::headers::{standard::{CONTENT_TYPE, DATE}, HeaderMap, HeaderValue};
    ///
    /// let mut map = HeaderMap::new();
    /// map.insert(CONTENT_TYPE, HeaderValue::from_static(b"text/html"));
    /// assert_eq!(map.get(CONTENT_TYPE).unwrap().as_str(), "text/html");
    ///
    /// let ctype = map.get(CONTENT_TYPE);
    /// ```
    #[inline]
    pub fn get(&self, name: &HeaderName) -> Option<&HeaderValue> {
        if self.is_empty() {
            return None;
        }
        Probe::new(self, name.hash()).get(name).map(HeaderField::value)
    }

    /// Returns an iterator to all header values corresponding to the given header name.
    ///
    /// Note that this is the result of duplicate header fields, *NOT* comma separated list.
    #[inline]
    pub fn get_all<'a>(&'a self, name: &'a HeaderName) -> GetAll<'a> {
        GetAll::new(self, name)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did have this key present, the value is updated, and the old value is returned
    /// as `Some`.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn insert(&mut self, name: HeaderName, value: HeaderValue) -> Option<HeaderField> {
        self.reserve_one().expect("cannot insert header");
        // SAFETY: `reserve_one` will make sure there is at least one remaining capacity
        unsafe { Probe::new(self, name.hash()).insert(self, HeaderField::new(name, value), false) }
    }

    /// Append a header key and value into the map.
    ///
    /// Unlike [`insert`][HeaderMap::insert], if header key is present, header value is still
    /// appended as extra value.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn append(&mut self, name: HeaderName, value: HeaderValue) {
        self.reserve_one().expect("cannot insert header");
        // SAFETY: `reserve_one` will make sure there is at least one remaining capacity
        unsafe { Probe::new(self, name.hash()).insert(self, HeaderField::new(name, value), true) };
    }

    pub(crate) fn try_append_field(
        &mut self,
        hash: u32,
        field: HeaderField,
    ) -> Result<(), TryReserveError> {
        self.reserve_one()?;
        // SAFETY: `reserve_one` will make sure there is at least one remaining capacity
        unsafe { Probe::new(self, hash).insert(self, field, true) };
        Ok(())
    }

    /// Removes a header from the map, returning the first header value if it founds.
    ///
    /// Note: Because this shifts over the remaining fields, it has significantly worse performance
    /// than other operations. If you don't need the order of fields to be preserved, use
    /// [`swap_remove`] instead.
    ///
    /// [`swap_remove`]: Self::swap_remove
    #[inline]
    pub fn remove(&mut self, name: &HeaderName) -> Option<HeaderField> {
        if self.is_empty() {
            return None;
        }
        Probe::new(self, name.hash()).remove(self, name)
    }

    /// Removes a header from the map, returning the first header value if it founds.
    ///
    /// The removed field is replaced by the last field of the map.
    ///
    /// This does not preserve ordering of the remaining fields. If you need to preserve the element
    /// order, use [`remove`] instead.
    ///
    /// [`remove`]: Self::remove
    #[inline]
    pub fn swap_remove(&mut self, name: &HeaderName) -> Option<HeaderField> {
        if self.is_empty() {
            return None;
        }
        Probe::new(self, name.hash()).swap_remove(self, name)
    }

    /// Reserves capacity for at least `additional` more headers.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.try_reserve(additional).expect("failed to reserve capacity")
    }

    /// Clear headers map, removing all the value.
    #[inline]
    pub fn clear(&mut self) {
        if self.is_empty() {
            return;
        }

        let offset = alloc::offset(self.cap) as usize;

        // clear the hash table
        unsafe { std::ptr::write_bytes(self.fields.as_ptr(), 0, offset) };

        // call drop on fields except the hash table
        unsafe {
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                self.fields.add(offset).as_ptr(),
                self.len as usize,
            ))
        };

        self.len = 0;
    }

    pub(crate) const fn fields(&self) -> &[HeaderField] {
        unsafe {
            slice::from_raw_parts(
                self.fields.add(alloc::offset(self.cap) as usize).as_ptr(),
                self.len as usize,
            )
        }
    }

    pub(crate) const fn fields_mut(&mut self) -> &mut [HeaderField] {
        unsafe {
            slice::from_raw_parts_mut(
                self.fields.add(alloc::offset(self.cap) as usize).as_ptr(),
                self.len as usize,
            )
        }
    }

    #[cfg(test)]
    const fn hash_table(&self) -> &[HashIdx] {
        unsafe {
            slice::from_raw_parts(
                self.fields.cast().as_ptr(),
                alloc::hash_field_cap(alloc::offset(self.cap)) as usize,
            )
        }
    }
}

// ===== Implementation =====

impl HeaderMap {
    #[inline]
    fn reserve_one(&mut self) -> Result<(), TryReserveError> {
        if !alloc::is_load_factor_exceeded(self.len, self.cap) {
            return Ok(());
        }
        self.try_reserve_size(1)
    }

    /// Reserves capacity for at least `additional` more headers.
    ///
    /// # Errors
    ///
    /// Returns error if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        let Ok(add) = Size::try_from(additional) else {
            return Err(TryReserveError {});
        };
        if self.cap - self.len > add {
            return Ok(());
        }
        let Some(new_cap) = self.len.checked_add(add) else {
            return Err(TryReserveError {});
        };
        self.try_reserve_size(new_cap)
    }

    fn try_reserve_size(&mut self, add: Size) -> Result<(), TryReserveError> {
        let Some(cap) = self
            .cap
            .max(alloc::MIN_CAP)
            .checked_mul(2)
            .max(self.len.checked_add(add))
        else {
            return Err(TryReserveError {});
        };

        let mut new_map = Self::with_capacity_size(cap);

        // copy to new map
        self.copy_to(&mut new_map);

        // skip drop, just deallocate
        if self.cap != 0 {
            alloc::deallocate(self.fields, self.cap);
        }
        let _ = mem::ManuallyDrop::new(mem::replace(self, new_map));

        Ok(())
    }

    fn copy_to(&self, new_map: &mut Self) {
        // recalculate hash table
        let ptr = self.fields.cast::<HashIdx>();
        let new_ptr = new_map.fields.cast::<HashIdx>();

        let offset = alloc::offset(self.cap);

        let new_offset = alloc::offset(new_map.cap);
        let new_hash_field_cap = alloc::hash_field_cap(new_offset) as Size;

        // when the hash table size changes, every index will also change
        let offset_delta = new_offset - offset;

        let mut i = 0;

        while new_map.len < self.len {
            let hash_field = unsafe { ptr.add(i as usize).as_ref() };
            let Some(hash_field_ref) = hash_field.as_ref() else {
                i += 1;
                continue;
            };

            let mut new_index = hash_field_ref.hash;
            loop {
                new_index %= new_hash_field_cap;
                let new_field = unsafe { new_ptr.add(new_index as usize).as_mut() };

                match new_field.as_mut() {
                    Some(_) => {
                        // collision
                        new_index += 1;
                    },
                    None => {
                        *new_field = Some(HashField {
                            hash: hash_field_ref.hash,
                            idx: unsafe { NonZeroU32::new_unchecked(hash_field_ref.idx.get() + offset_delta) },
                        });
                        new_map.len += 1;
                        break;
                    }
                }
            }

            i += 1;
        }

        // copy all fields
        unsafe {
            self.fields
                .add(offset as usize)
                .copy_to_nonoverlapping(new_map.fields.add(new_offset as usize), self.len as usize)
        };

        debug_assert_eq!(new_map.len, self.len);
    }
}

// ===== Probe Search =====

/// The implementation of this header map is based on two list, the hash table and header fields.
///
/// `HeaderMap` is the compact representation, while `Probe` is the loose representation. It stores
/// necessary fields that require computation to retrieve.
///
/// `Probe` only handle hash table operations, allocation should be handled by `HeaderMap`.
#[derive(Clone)]
struct Probe {
    ptr: ptr::NonNull<HashIdx>,
    /// `>= alloc::MIN_CAP`
    offset: u32,
    hash_field_cap: u32,

    index: u32,
    hash: u32,
}

impl Probe {
    fn new(map: &HeaderMap, hash: u32) -> Self {
        debug_assert!(map.cap >= alloc::MIN_CAP);
        let offset = alloc::offset(map.cap);
        let hash_field_cap = alloc::hash_field_cap(offset);
        Self {
            ptr: map.fields.cast::<HashIdx>(),
            offset,
            hash_field_cap,
            index: hash % hash_field_cap,
            hash,
        }
    }

    fn hash_field<'a>(&self) -> Option<&'a HashField> {
        unsafe { self.ptr.add(self.index as usize).as_ref().as_ref() }
    }

    fn hash_field_mut<'a>(&self) -> &'a mut Option<HashField> {
        unsafe { self.ptr.add(self.index as usize).as_mut() }
    }

    fn advance(&mut self) {
        self.index = (self.index + 1) % self.hash_field_cap;
    }
}

// ===== Implementation =====

impl Probe {
    fn get<'a>(mut self, name: &HeaderName) -> Option<&'a HeaderField> {
        loop {
            let hash_field = self.hash_field()?;
            if let Some(field) = hash_field.checked_field(&self, name) {
                return Some(field);
            }
            self.advance();
        }
    }

    /// # Safety
    ///
    /// Remaining allocation must be able to holds one more field.
    unsafe fn insert(
        mut self,
        map: &mut HeaderMap,
        field: HeaderField,
        append: bool,
    ) -> Option<HeaderField> {
        loop {
            let hash_field_mut = self.hash_field_mut();
            let Some(dup_hash_field) = hash_field_mut.as_mut() else {
                // found empty slot

                // SAFETY: by fn safety, capacity is non-zero, thus `offset` will never be zero
                let idx = unsafe { NonZeroU32::new_unchecked(map.len + self.offset) };
                let hash = self.hash;
                let hash_field = HashField { hash, idx };

                // write the header field and hash field
                // SAFETY: by fn safety, there must be at least one remaining capacity
                unsafe { hash_field.put_field(&self, field) };
                *hash_field_mut = Some(hash_field);

                // update length
                map.len += 1;

                return None;
            };

            if !append
                && let Some(dup_field) = dup_hash_field.checked_field_mut(&self, field.name())
            {
                // if it the same name, replace and returns the duplicate
                return Some(mem::replace(dup_field, field));
            }

            // appending, look for the next empty slot
            self.advance();
        }
    }

    fn remove(mut self, map: &mut HeaderMap, name: &HeaderName) -> Option<HeaderField> {
        // 1. find the target hash field
        loop {
            let hash_field = self.hash_field_mut().as_mut()?;
            if hash_field.checked_field(&self, name).is_some() {
                break;
            }
            self.advance();
        }
        let hash_field_idx = self.index;

        // 2. replace the hash field with other entry that may be displaced by collision
        let mut swap_candidate = None;
        loop {
            self.advance();
            // SAFETY: `index` is masked by hash table capacity
            let swap_hfield_mut = self.hash_field_mut();
            let Some(swap_hfield) = swap_hfield_mut.as_ref() else {
                break;
            };
            // only hash field that is displaced that should be swapped, other field may just
            // happens to be contiguous
            if swap_hfield.hash % self.hash_field_cap == hash_field_idx {
                swap_candidate = swap_hfield_mut.take();
            }
        }
        // SAFETY:
        // - `hash_field_idx` is result of a search, its valid index
        // - `unwrap_unchecked` is safe because if the target hash field is `None`, this function
        // already returned
        let hash_field = unsafe {
            self
                .ptr
                .add(hash_field_idx as usize)
                .replace(swap_candidate)
                .unwrap_unchecked()
        };

        // 3. update hash fields indexes that will be shifted
        // `hash_field.idx` are index with `offset` applied, but `self.len` is not
        let max_bounds = map.len + self.offset;
        let mut i = hash_field.idx.get() + 1;
        while i < max_bounds {
            let field_mut = unsafe { self.ptr.cast::<HeaderField>().add(i as usize).as_mut() };
            let name = field_mut.name();
            let hash = name.hash();
            self.index = hash % self.hash_field_cap;
            self.hash = hash;

            loop {
                // let hash_field = unsafe { ptr.add(hash_idx as usize).as_mut().as_mut() };
                let Some(hash_field) = self.hash_field_mut() else {
                    unreachable!("fields with no hash entry")
                };
                // check whether this is the correct hash fields, not displaced
                if hash_field.checked_field(&self, name).is_some() {
                    // affected fields is backshifted
                    hash_field.idx = unsafe { NonZeroU32::new_unchecked(i - 1) };
                    break;
                }
                // displaced, search next entry
                self.advance();
            }

            i += 1;
        }

        // 4. take out the removed field
        let field_ptr = hash_field.field_ptr(&self);
        // the ptr here will be overwritten
        let field = unsafe { field_ptr.read() };

        // 5. copy the fields backward
        // do this AFTER all hash tables updated
        let backshift_len = map.len - (hash_field.idx.get() - self.offset);
        unsafe { field_ptr.copy_from(field_ptr.add(1), backshift_len as usize) };

        // update the length
        map.len -= 1;
        Some(field)
    }

    fn swap_remove(mut self, map: &mut HeaderMap, name: &HeaderName) -> Option<HeaderField> {
        // 1. find the target hash field
        loop {
            let hash_field = self.hash_field_mut().as_mut()?;
            if hash_field.checked_field(&self, name).is_some() {
                break;
            }
            self.advance();
        }
        let hash_field_idx = self.index;

        // 2. replace the hash field with other entry that may be displaced by collision
        let mut swap_candidate = None;
        loop {
            self.advance();
            // SAFETY: `index` is masked by hash table capacity
            let hash_field_mut = self.hash_field_mut();
            let Some(hash_field) = hash_field_mut.as_mut() else {
                break;
            };
            // only hash field that is displaced that should be swapped, other field may just
            // happens to be contiguous
            if hash_field.hash % self.hash_field_cap == hash_field_idx {
                swap_candidate = hash_field_mut.take();
            }
        }
        // SAFETY:
        // - `target_idx` is result of the search, its valid index
        // - `unwrap_unchecked` is safe because if the target hash field is `None`, this function
        // already returned
        let hash_field = unsafe {
            self.ptr
                .add(hash_field_idx as usize)
                .replace(swap_candidate)
                .unwrap_unchecked()
        };

        // 3. update the last field's hash field index
        debug_assert!(!map.is_empty());
        let last_field_idx = (self.offset + (map.len - 1)) as usize;
        if map.len != 1 {
            let last_field = unsafe { self.ptr.cast::<HeaderField>().add(last_field_idx).as_mut() };
            let name = last_field.name();
            let hash = name.hash();
            self.index = hash % self.hash_field_cap;
            self.hash = hash;

            loop {
                let Some(last_hash_field) = self.hash_field_mut() else {
                    unreachable!("fields with no hash entry")
                };
                // check whether this is the correct hash fields, not displaced
                if last_hash_field.checked_field(&self, name).is_some() {
                    last_hash_field.idx = hash_field.idx;
                    break;
                }
                // displaced, search next entry
                self.advance();
            }
        }

        // 4. take out the removed field
        let field_ptr = hash_field.field_ptr(&self);
        // if there is last element, this will be overwritten,
        // otherwise, the len will be 0, thus no drop will be called
        let field = unsafe { field_ptr.read() };

        // 5. replace with the last field
        if map.len != 1 {
            unsafe {
                self.ptr
                    .cast::<HeaderField>()
                    .add(last_field_idx)
                    .copy_to(field_ptr, 1)
            }
        }

        // update the length
        map.len -= 1;
        Some(field)
    }
}

// ===== std traits =====

impl std::fmt::Debug for HeaderMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.pairs()).finish()
    }
}

// ===== Duplicate Header Values Iterator =====

/// An immutable iterator over the header values with the same header name.
///
/// This iterator is created from [`HeaderMap::get_all`] method.
#[derive(Clone)]
pub struct GetAll<'a> {
    name: &'a HeaderName,
    probe: Probe,
}

impl<'a> GetAll<'a> {
    #[inline]
    fn new(map: &'a HeaderMap, name: &'a HeaderName) -> Self {
        Self { name, probe: Probe::new(map, name.hash()) }
    }
}

impl<'a> Iterator for GetAll<'a> {
    type Item = &'a HeaderValue;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let hash_field = self.probe.hash_field()?;
            self.probe.advance();
            if hash_field.hash != self.probe.hash {
                continue;
            }
            let field = hash_field.field(&self.probe);
            if field.name() != self.name {
                continue;
            }
            break Some(field.value())
        }
    }
}

impl<'a> std::fmt::Debug for GetAll<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a> IntoIterator for &'a HeaderMap {
    type Item = &'a HeaderField;

    type IntoIter = slice::Iter<'a, HeaderField>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ===== Allocation =====

mod alloc {
    //! Allocation for the HeaderMap is divided into two region. The first region is used to store
    //! hash and index pair for lookups, then the rest is where the fields are stored.
    //!
    //! ```not_rust
    //! SIZE = 48
    //! LOAD_FACTOR = 3/4
    //! load = cap * LOAD_FACTOR
    //! off = cap * (1 - LOAD_FACTOR)
    //! [ off | load ]
    //! ```

    use std::alloc::{Layout, handle_alloc_error, alloc, dealloc};
    use std::ptr::NonNull;

    use super::{HeaderField, HashIdx, Size};

    // const LOAD_FACTOR: f32  = 3 / 4;

    pub const MIN_CAP: u32 = 2;

    const HASH_SIZE: usize = size_of::<HashIdx>();
    pub const SIZE: usize = size_of::<HeaderField>();
    pub const ALIGN: usize = align_of::<HeaderField>();

    // how many hash field can be stored in one SIZE.
    pub const OFFSET_SCALE: usize = SIZE / HASH_SIZE;

    // no allocation overflow
    const _: () = assert!(((Size::MAX as usize).strict_mul(SIZE)) < isize::MAX as usize);

    // unused capacity in remaining of the load factor is enough to store hash table
    const _: () = assert!(offset(3) as usize * HASH_SIZE <= SIZE * 3);

    /// Calculate offset to the first pointer of the fields.
    ///
    /// Returned `offset` is in [`SIZE`] bytes.
    pub const fn offset(cap: Size) -> u32 {
        // cap * (1 - LOAD_FACTOR)
        cap / 4
    }

    /// Calculate capacity of hash table.
    pub const fn hash_field_cap(offset: u32) -> u32 {
        offset * OFFSET_SCALE as u32
    }

    pub const fn is_load_factor_exceeded(len: Size, cap: Size) -> bool {
        // more optimized of `self.len as f64 / self.cap as f64 >= LOAD_FACTOR`
        // this also handle 0 capacity
        len * 4 >= cap * 3
    }

    const fn layout(cap: Size) -> Layout {
        // `Size::MAX * SIZE` is below `isize::MAX`
        unsafe { Layout::from_size_align_unchecked((cap as usize).unchecked_mul(SIZE), ALIGN) }
    }

    pub fn allocate(cap: Size) -> NonNull<HeaderField> {
        unsafe {
            let layout = layout(cap);
            match NonNull::new(alloc(layout)) {
                Some(ok) => ok.cast(),
                None => handle_alloc_error(layout),
            }
        }
    }

    pub fn allocate_init(cap: Size) -> NonNull<HeaderField> {
        let ptr = allocate(cap);
        // initialized the hash table
        unsafe { std::ptr::write_bytes(ptr.as_ptr(), 0, offset(cap) as usize) };
        ptr
    }

    pub fn deallocate(ptr: NonNull<HeaderField>, cap: Size) {
        unsafe { dealloc(ptr.cast().as_ptr(), layout(cap)) };
    }
}

#[cfg(test)]
#[allow(clippy::borrow_interior_mutable_const)]
#[allow(clippy::declare_interior_mutable_const)]
mod test {
    use std::ptr::NonNull;
    use crate::headers::name::standard as s;
    use super::*;

    macro_rules! assert_field {
        ($map:ident, $fields:expr) => {
            assert!(
                $map
                    .fields()
                    .iter()
                    .map(|e| e.name())
                    .zip($fields)
                    .all(|(m, n)| m == n)
            );
        };
    }

    #[allow(unused)]
    macro_rules! dbg_map {
        ($map:ident, $($tt:tt)*) => {{
            println!("==={}({})===", $($tt)*, $map.len());
            let offset = alloc::offset($map.cap);
            let cap = alloc::hash_field_cap(offset);

            for f in $map.hash_table() {
                print!("{f:?}");
                if let Some(f) = f {
                    print!(" ({})", f.hash % cap as u32);
                }
                println!();
            }
            for f in $map.fields() {
                let hash = f.name().hash();
                println!("{f:?} ({hash}#{})", hash % cap as u32);
            }
            println!("=====");
        }};
    }

    const FOO: HeaderValue = HeaderValue::from_static(b"FOO");

    const fn is_send_sync<T: Send + Sync>() {}
    const _: () = is_send_sync::<HeaderMap>();

    #[test]
    fn test_zeroed_hash_idx() {
        let mut mem = HeaderField::new(s::HOST, FOO);

        let ptr = NonNull::from_mut(&mut mem);

        unsafe {
            // `allocate` use zero bytes write to initialized the hash table
            std::ptr::write_bytes(ptr.as_ptr(), 0, 1);

            for i in 0..alloc::OFFSET_SCALE {
                assert_eq!(ptr.cast::<HashIdx>().add(i).as_ref(), &None);
            }
        }
    }

    #[test]
    fn test_empty() {
        // dangling ptr
        drop(HeaderMap::new());

        drop(HeaderMap::with_capacity_size(8));

        let mut map = HeaderMap::new();
        map.reserve(7);
    }

    #[test]
    fn test_insert() {
        test_insert_impl(&mut HeaderMap::new());
        test_insert_impl(&mut HeaderMap::with_capacity(8));
    }

    #[test]
    fn test_append() {
        let mut map = HeaderMap::new();
        map.append(s::DATE, FOO);
        map.append(s::DATE, FOO);
        assert_eq!(map.get_all(&s::DATE).count(), 2);
        assert_eq!(map.len(), 2);
        assert_field!(map, &[s::DATE, s::DATE]);
    }

    #[test]
    fn test_remove() {
        let mut map = HeaderMap::new();
        map.insert(s::DATE, FOO);
        test_remove_impl(&mut map);
        drop(map);

        let mut map = build_map();
        test_remove_impl(&mut map);
        drop(map);

        let mut map = build_map();
        map.append(s::DATE, FOO);
        test_remove_impl(&mut map);
        drop(map);
    }

    #[test]
    fn test_swap_remove() {
        let mut map = HeaderMap::new();
        map.insert(s::DATE, FOO);
        test_swap_remove_impl(&mut map);
        drop(map);

        let mut map = build_map();
        test_swap_remove_impl(&mut map);
        drop(map);

        let mut map = build_map();
        map.append(s::DATE, FOO);
        test_swap_remove_impl(&mut map);
        drop(map);
    }

    fn build_map() -> HeaderMap {
        let mut map = HeaderMap::new();
        test_insert_impl(&mut map);
        map
    }

    fn test_insert_impl(map: &mut HeaderMap) {
        const NAMES: &[HeaderName] = &[
            s::ACCEPT,
            s::AGE,
            s::ALLOW,
            s::COOKIE,
            s::CONTENT_LENGTH,
            s::CONTENT_TYPE,
            s::DATE,
            s::HOST,
            s::TE,
            s::USER_AGENT,
        ];

        for name in NAMES {
            assert!(map.insert(name.clone(), FOO).is_none());
            assert!(map.contains_key(name));
            map.fields();
            map.hash_table();
        }

        assert_field!(map, NAMES);
    }

    fn test_remove_impl(map: &mut HeaderMap) {
        let len = map.len();
        let field = map.remove(&s::DATE).unwrap();
        assert_eq!(field.name(), &s::DATE);
        assert_eq!(field.value(), &FOO);
        assert_eq!(map.len(), len - 1);
    }

    fn test_swap_remove_impl(map: &mut HeaderMap) {
        let len = map.len();
        let field = map.swap_remove(&s::DATE).unwrap();
        assert_eq!(field.name(), &s::DATE);
        assert_eq!(field.value(), &FOO);
        assert_eq!(map.len(), len - 1);
    }
}

