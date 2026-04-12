use crate::Scheme;
use crate::body::shared::SendHandle;
use crate::headers::HeaderMap;

#[derive(Debug)]
pub struct Session {
    pub scheme: Scheme,
    pub headers: HeaderMap,
    pub shared: SendHandle,
    pub keep_alive: bool,
}

impl Session {
    pub fn new() -> Self {
        Self {
            scheme: Scheme::HTTP,
            headers: HeaderMap::with_capacity(32),
            shared: SendHandle::new(),
            keep_alive: true,
        }
    }
}
