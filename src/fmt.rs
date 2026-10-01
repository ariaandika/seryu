pub use core::fmt::*;

pub fn debug_ascii(bytes: &[u8]) -> impl Debug {
    // `<[u8]>::escape_ascii` only works with `Display`, so to provide str-like `Debug` it needs to
    // be surrounded by quote
    from_fn(|f| Display::fmt(&format_args!("\"{}\"", bytes.escape_ascii()), f))
}
