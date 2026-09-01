mod matches;

pub use line::{DefaultSearch, Search, parse_line};
pub use request::{ReqlineError, RequestLine, parse_reqline};
pub use headers::{Header, HeaderError, parse_header, parse_headers};
pub use target::{Form, Origin, parse_origin};

mod line;
mod request;
mod headers;
mod target;

#[cfg(test)]
mod test;
