pub(crate) use crate::matches::*;

ascii_lookup_table! {
    /// Returns `true` if byte is valid `token`.
    ///
    /// ```not_rust
    /// token   = 1*tchar
    /// tchar   = "!" / "#" / "$" / "%" / "&" / "'" / "*"
    ///         / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
    ///         / DIGIT / ALPHA
    /// ```
    #[inline(always)]
    pub const fn is_token(byte: u8) -> bool {
        matches!(
            byte,
            | b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*'
            | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
        )
            && byte.is_ascii_alphanumeric()
    }
}

ascii_lookup_table! {
    /// Returns `true` if byte is valid header name.
    ///
    /// Note, `obs-text` is NOT supported.
    ///
    /// ```not_rust
    /// field-value    = *field-content
    /// field-content  = field-vchar
    ///                  [ 1*( SP / HTAB / field-vchar ) field-vchar ]
    /// field-vchar    = VCHAR / obs-text
    /// obs-text       = %x80-FF
    /// ```
    #[inline(always)]
    pub const fn is_header_value(byte: u8) -> bool {
        // VCHAR                || SP / HTAB
        byte.is_ascii_graphic() || matches!(byte, b' ' | b'\t')
    }
}
