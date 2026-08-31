use core::mem::MaybeUninit;

use crate::h1::matches;

// ===== Search =====

/// Bytes searching implementation.
///
/// Use [`DefaultSearch`] to use default sequential searching.
///
/// # Safety
///
/// [`Search::find`] must returns index within bounds of `bytes`.
pub unsafe trait Search {
    /// Search `byte` in `bytes` and returns the index.
    fn find(bytes: &[u8], byte: u8) -> Option<usize>;

    #[inline]
    fn split_byte(bytes: &[u8], byte: u8) -> Option<(&[u8], &[u8])> {
        let pos = Self::find(bytes, byte)?;
        // SAFETY: `Search` implementation guarantee that the index is in bounds
        unsafe { Some(bytes.split_at_unchecked(pos + 1)) }
    }
}

/// Sequential search implementation of [`Search`].
#[derive(Debug)]
pub struct DefaultSearch;

unsafe impl Search for DefaultSearch {
    #[inline]
    fn find(bytes: &[u8], byte: u8) -> Option<usize> {
        bytes.iter().position(|&b| b == byte)
    }
}

// ===== Header =====

#[derive(Debug)]
pub struct Header<'a> {
    pub name: &'a [u8],
    pub value: &'a [u8],
}

impl<'a> Header<'a> {
    #[inline]
    pub fn parse(header: &'a [u8]) -> Result<Self, HeaderError> {
        let mut me = MaybeUninit::uninit();
        parse_header::<DefaultSearch>(header, &mut me)?;
        // SAFETY: `parse_reqline` guarantee that its initialized
        Ok(unsafe { me.assume_init() })
    }
}

#[inline]
pub const fn parse_header<'a, S: Search>(
    header: &'a [u8],
    output: &mut MaybeUninit<Header<'a>>,
) -> Result<(), HeaderError> {
    let Some((name, val)) = matches::split_to_delim(header, b':') else {
        return Err(HeaderError::InvalidSeparator);
    };
    matches::write_field!(output.name, name);
    matches::write_field!(output.value, val.trim_ascii_start());
    Ok(())
}

// ===== Headers =====

#[derive(Debug)]
pub struct Headers<'a, 'b> {
    bytes: &'b [u8],
    headers: &'a mut [MaybeUninit<Header<'b>>],
    len: usize,
}

impl<'a, 'b> Headers<'a, 'b> {
    /// Creates new [`Headers`].
    #[inline]
    pub fn new(bytes: &'b [u8], headers: &'a mut [MaybeUninit<Header<'b>>]) -> Self {
        Self { bytes, headers, len: 0 }
    }

    /// Returns the raw headers bytes.
    #[inline]
    pub fn bytes(&self) -> &'b [u8] {
        self.bytes
    }

    /// Returns the initialized headers.
    ///
    /// Note that after creating this struct, [`Headers::parse`] must be called to start parsing and
    /// initializing the headers.
    #[inline]
    pub fn headers(&self) -> &'a [Header<'b>] {
        unsafe { &*(self.headers.get_unchecked(..self.len) as *const _ as *const [Header<'b>]) }
    }

    /// Parse headers, returning how many bytes was consumed.
    ///
    /// Use [`Headers::headers`] to get the parsed headers.
    pub fn parse<S: Search>(&mut self) -> Result<usize, HeaderError> {
        let mut headers = self.bytes;
        self.len = 0;

        loop {
            let Some((line, rest)) = S::split_byte(headers, b'\n') else {
                return Err(HeaderError::InsufficientBytes);
            };
            headers = rest;

            let header = line.trim_ascii_end();
            if header.is_empty() {
                break;
            }

            let Some(output) = self.headers.get_mut(self.len) else {
                return Err(HeaderError::InsufficientBuf);
            };
            output.write(Header::parse(header)?);

            self.len += 1;
        }

        Ok(headers.as_ptr().addr() - self.bytes.as_ptr().addr())
    }
}

// ===== errors =====

#[derive(Debug)]
pub enum HeaderError {
    InsufficientBytes,
    InsufficientBuf,
    InvalidSeparator,
}
