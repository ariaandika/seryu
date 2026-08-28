use core::num::NonZeroU16;

/// HTTP Status Code.
///
/// This API implements status codes defined in [RFC9110] and [RFC6585], [451 (Unavailable For
/// Legal Reasons)][RFC7725], and [103 (Early Hints)][RFC8297].
///
/// [RFC9110]: <https://www.rfc-editor.org/rfc/rfc9110#name-status-codes>
/// [RFC6585]: <https://www.rfc-editor.org/rfc/rfc6585>
/// [RFC7725]: <https://www.rfc-editor.org/rfc/rfc7725>
/// [RFC8297]: <https://www.rfc-editor.org/rfc/rfc8297>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(NonZeroU16);

impl Default for StatusCode {
    #[inline]
    fn default() -> Self {
        Self::OK
    }
}

impl StatusCode {
    /// Returns status code number as `u16`.
    #[inline]
    pub const fn as_u16(&self) -> u16 {
        self.0.get()
    }

    /// Returns status code number as str.
    #[inline]
    pub const fn code_str(&self) -> &'static str {
        build_str!(self 3)
    }

    /// Returns status code reason as str.
    #[inline]
    pub const fn reason(&self) -> &'static str {
        build_str!(self,+ 4|len|)
    }

    /// Returns status code number and message as string.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        build_str!(self|len|)
    }

    /// Returns `true` if status code class is informational.
    ///
    /// Informational means the request was received, continuing process.
    #[inline]
    pub const fn is_informational(&self) -> bool {
        self.0.get() / 100 == 1
    }

    /// Returns `true` is status code class is successful.
    ///
    /// Successful means the request was successfully received, understood, and accepted.
    #[inline]
    pub const fn is_successful(&self) -> bool {
        self.0.get() / 100 == 2
    }

    /// Returns `true` is status code class is redirection.
    ///
    /// Redirection means further action needs to be taken in order to complete the request.
    #[inline]
    pub const fn is_redirection(&self) -> bool {
        self.0.get() / 100 == 3
    }

    /// Returns `true` is status code class is client error.
    ///
    /// Client error means the request contains bad syntax or cannot be fulfilled.
    #[inline]
    pub const fn is_client_error(&self) -> bool {
        self.0.get() / 100 == 4
    }

    /// Returns `true` is status code class is server error.
    ///
    /// Server error means the server failed to fulfill an apparently valid request.
    #[inline]
    pub const fn is_server_error(&self) -> bool {
        self.0.get() / 100 == 5
    }
}

/// 100, 200, 300, and 500 status codes have at most 12 elements each.
///
/// Divide the table into segments with that elements each.
///
/// Status code is modulused by 500 so that 500 status codes use the first segment of the table.
///
/// `400` status codes have at most 32 elements, this will overflow the segment, but its fine
/// because it is the last segment, thus no overlap.
///
/// In the table, each entry stores an offset to the start of reason string, and the next entry
/// can be used to get the length of the reason string.
///
/// Because 500 status codes wrapped to the first segment, it will result in wrong length when
/// collided with 100 status code, therefore segment size should be most elements count plus one.
const fn status_to_index(status: u16) -> u16 {
    let status = status % 500;
    ((status / 100) * 13) + (status % 100)
}

/// `(status, reason_offset)`
static TABLE: [(u16, u16); 88] = {
    let mut table = [(0, 0); 88];
    let mut table_i = 0;
    let mut values_i = 0;

    let table_len = table.len();

    while table_i < table_len {
        let (status, reason) = VALUES[values_i];
        if status / 100 == 5 {
            break;
        }

        let idx = status_to_index(status);
        let current_mut = &mut table[table_i];

        if table_i == idx as usize {
            let offset = current_mut.1 + reason.len() as u16 + 4;

            current_mut.0 = status;
            table[table_i + 1].1 = offset;

            values_i += 1;
        } else {
            // padding entry
            table[table_i + 1].1 = current_mut.1;
        }

        table_i += 1;
    }

    table[0].1 = table[table_i].1;

    // special case for 500 status codes that placed in the first segment
    table_i = 0;
    while values_i < VALUES.len() {
        let (status, reason) = VALUES[values_i];
        let idx = status_to_index(status);

        let current_mut = &mut table[table_i];

        if table_i == idx as usize {
            let offset = current_mut.1 + reason.len() as u16 + 4;

            current_mut.0 = status;
            if table_i + 1 < table_len {
                table[table_i + 1].1 = offset;
            }

            values_i += 1;
        } else {
            // padding entry
            if table_i + 1 < table_len {
                table[table_i + 1].1 = current_mut.1;
            }
        }

        table_i += 1;
    }

    table
};

macro_rules! build_str {
    (
        $me:ident
        $(,$off1:tt $off2:literal)?
        $(|$len:ident|)?
        $($len_lit:literal)?
    ) => {
        unsafe {
            let index = status_to_index($me.0.get()) as usize;

            // SAFETY: valid status will always result in bounds index
            let offset = ((*TABLE.as_ptr().add(index)).1 $($off1 $off2)?);

            // SAFETY: highest status will not resulting in last element, there is always
            // next element
            $(
                let end = (*TABLE.as_ptr().add(index.unchecked_add(1))).1;
                let $len = end.unchecked_sub(offset);
            )?

            str::from_utf8_unchecked(core::slice::from_raw_parts(
                REASONS.as_ptr().add(offset as usize),
                $($len as usize)? $($len_lit)?
            ))
        }
    };
}

use build_str;

macro_rules! status_code {
    ($(
        $(#[$doc:meta])*
        $int:literal $id:ident $msg:literal;
    )*) => {
        impl StatusCode {
            $(
                $(#[$doc])*
                pub const $id: Self = Self(NonZeroU16::new($int).unwrap());
            )*
        }
        const VALUES: [(u16, &str); 48] = [$(($int,$msg)),*];
        static REASONS: &[u8] = concat!($(concat!(stringify!($int)," ",$msg)),*).as_bytes();
    };
}
status_code! {
    /// The `100` (Continue) status code.
    100 CONTINUE "Continue";
    /// The `101` (Switching Protocols) status code.
    101 SWITCHING_PROTOCOL "Switching Protocols";
    /// The `103` (Early Hints) status code.
    103 EARLY_HINTS "Early Hints";
    /// The `200` (OK) status code.
    200 OK "OK";
    /// The `201` (Created) status code.
    201 CREATED "Created";
    /// The `202` (Accepted) status code.
    202 ACCEPTED "Accepted";
    /// The `203` (Non-Authoritative Information) status code.
    203 NON_AUTHORATIVE_INFORMATION "Non-Authoritative Information";
    /// The `204` (No Content) status code.
    204 NO_CONTENT "No Content";
    /// The `205` (Reset Content) status code.
    205 RESET_CONTENT "Reset Content";
    /// The `206` (Partial Content) status code.
    206 PARTIAL_CONTENT "Partial Content";
    /// The `300` (Multiple Choices) status code.
    300 MULTIPLE_CHOICES "Multiple Choices";
    /// The `301` (Moved Permanently) status code.
    301 MOVED_PERMANENTLY "Moved Permanently";
    /// The `302` (Found) status code.
    302 FOUND "Found";
    /// The `303` (See Other) status code.
    303 SEE_OTHER "See Other";
    /// The `304` (Not Modified) status code.
    304 NOT_MODIFIED "Not Modified";
    /// The `307` (Temporary Redirect) status code.
    307 TEMPORARY_REDIRECT "Temporary Redirect";
    /// The `308` (Permanent Redirect) status code.
    308 PERMANENT_REDIRECT "Permanent Redirect";
    /// The `400` (Bad Request) status code.
    400 BAD_REQUEST "Bad Request";
    /// The `401` (Unauthorized) status code.
    401 UNAUTHORIZED "Unauthorized";
    // The 402 (Payment Required) status code is reserved for future use.
    /// The `403` (Forbidden) status code.
    403 FORBIDDEN "Forbidden";
    /// The `404` (Not Found) status code.
    404 NOT_FOUND "Not Found";
    /// The `405` (Method Not Allowed) status code.
    405 METHOD_NOT_ALLOWED "Method Not Allowed";
    /// The `406` (Not Acceptable) status code.
    406 NOT_ACCEPTABLE "Not Acceptable";
    /// The `407` (Proxy Authentication Required) status code.
    407 PROXY_AUTHENTICATION_REQUIRED "Proxy Authentication Required";
    /// The `408` (Request Timeout) status code.
    408 REQUEST_TIMEOUT "Request Timeout";
    /// The `409` (Conflict) status code.
    409 CONFLICT "Conflict";
    /// The `410` (Gone) status code.
    410 GONE "Gone";
    /// The `411` (Length Required) status code.
    411 LENGTH_REQUIRED "Length Required";
    /// The `412` (Precondition Failed) status code.
    412 PRECONDITION_FAILED "Precondition Failed";
    /// The `413` (Content Too Large) status code.
    413 CONTENT_TOO_LARGE "Content Too Large";
    /// The `414` (URI Too Long) status code.
    414 URI_TOO_LONG "URI Too Long";
    /// The `415` (Unsupported Media Type) status code.
    415 UNSUPPORTED_MEDIA_TYPE "Unsupported Media Type";
    /// The `416` (Range Not Satisfiable) status code.
    416 RANGE_NOT_SATISFIABLE "Range Not Satisfiable";
    /// The `417` (Expectation Failed) status code.
    417 EXPECTATION_FAILED "Expectation Failed";
    /// The `418` (I'm a teapot) status code.
    418 IM_A_TEAPOT "I'm a teapot";
    /// The `421` (Misdirected Request) status code.
    421 MISDIRECTED_REQUEST "Misdirected Request";
    /// The `422` (Unprocessable Content) status code.
    422 UNPROCESSABLE_CONTENT "Unprocessable Content";
    /// The `426` (Upgrade Required) status code.
    426 UPGRADE_REQUIRED "Upgrade Required";
    /// The `428` (Precondition Required) status code.
    428 PRECONDITION_REQUIRED "Precondition Required";
    /// The `429` (Too Many Requests) status code.
    429 TOO_MANY_REQUESTS "Too Many Requests";
    /// The `431` (Request Header Fields Too Large) status code.
    431 REQUEST_HEADER_FIELDS_TOO_LARGE "Request Header Fields Too Large";
    /// The `500` (Internal Server Error) status code.
    500 INTERNAL_SERVER_ERROR "Internal Server Error";
    /// The `501` (Not Implemented) status code.
    501 NOT_IMPLEMENTED "Not Implemented";
    /// The `502` (Bad Gateway) status code.
    502 BAD_GATEWAY "Bad Gateway";
    /// The `503` (Service Unavailable) status code.
    503 SERVICE_UNAVAILABLE "Service Unavailable";
    /// The `504` (Gateway Timeout) status code.
    504 GATEWAY_TIMEOUT "Gateway Timeout";
    /// The `505` (HTTP Version Not Supported) status code.
    505 HTTP_VERSION_NOT_SUPPORTED "HTTP Version Not Supported";
    /// The `511` (Network Authentication Required) status code.
    511 NETWORK_AUTHENTICATION_REQUIRED "Network Authentication Required";
}
