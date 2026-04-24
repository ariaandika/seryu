use crate::request::RequestHead;

#[derive(Debug)]
pub struct Request<B> {
    head: RequestHead,
    body: B,
}

impl<B> Request<B> {
    pub const fn new(head: RequestHead, body: B) -> Self {
        Self { head, body }
    }

    pub const fn head(&self) -> &RequestHead {
        &self.head
    }

    pub const fn head_mut(&mut self) -> &mut RequestHead {
        &mut self.head
    }

    pub const fn body(&self) -> &B {
        &self.body
    }

    pub const fn body_mut(&mut self) -> &mut B {
        &mut self.body
    }

    pub fn into_parts(self) -> (RequestHead, B) {
        (self.head, self.body)
    }
}
