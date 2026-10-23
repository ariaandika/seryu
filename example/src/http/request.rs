use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::slice;

use seryu::h1;

const MAX_HEADERS: usize = 24;

// ===== Request =====

pub struct Request {
    // bounded to task lifetime
    data: NonNull<RequestData>,
}

impl Request {
    pub(super) fn new(data: &RequestData) -> Self {
        Self { data: NonNull::from_ref(data) }
    }

    pub fn method(&self) -> &[u8] {
        unsafe { self.data.as_ref().method.as_ref() }
    }

    pub fn target(&self) -> &[u8] {
        unsafe { self.data.as_ref().target.as_ref() }
    }

    pub fn headers(&self) -> &[h1::Field<'_>] {
        unsafe {
            let me = self.data.as_ref();
            slice::from_raw_parts(&raw const me.headers as _, me.len)
        }
    }
}

// ===== RequestData =====

type RawHeader = (NonNull<[u8]>, NonNull<[u8]>);

#[repr(C)]
pub struct RequestData {
    method: NonNull<[u8]>,
    target: NonNull<[u8]>,
    len: usize,
    headers: [RawHeader; MAX_HEADERS],
}

impl RequestData {
    pub(super) fn parse(
        mut bytes: &[u8],
        request: &mut MaybeUninit<RequestData>,
    ) -> Result<(), h1::ParseError> {
        // find request line delimiter
        let Some(line) = h1::read_line(&mut bytes) else {
            return Err(h1::ParseError::InsufficientBytes);
        };

        // parse request
        let req = h1::RequestLine::parse(line)?;
        let ptr = request.as_mut_ptr();
        unsafe {
            (&raw mut (*ptr).method).write(req.method.into());
            (&raw mut (*ptr).target).write(req.target.into());
        }

        // headers
        let len = unsafe { &mut (*ptr).len };
        *len = 0;

        loop {
            // find header line delimiter
            let Some(line) = h1::read_line(&mut bytes) else {
                return Err(h1::ParseError::InsufficientBytes);
            };
            if line.is_empty() {
                break;
            }
            if *len == MAX_HEADERS {
                return Err(h1::ParseError::ExcessiveHeaders);
            }

            // parse header
            let header = h1::Field::parse(line)?;
            unsafe {
                (&raw mut (*request.as_mut_ptr()).headers)
                    .cast::<RawHeader>()
                    .add(*len)
                    .write((header.name.into(), header.value.into()));
            }

            *len += 1;
        }

        Ok(())
    }
}
