use crate::response::ResponseHead;

#[derive(Debug)]
pub struct Response<B> {
    head: ResponseHead,
    body: B,
}

impl<B> Response<B> {
    pub const fn new(head: ResponseHead, body: B) -> Self {
        Self { head, body }
    }

    pub const fn head(&self) -> &ResponseHead {
        &self.head
    }

    pub const fn head_mut(&mut self) -> &mut ResponseHead {
        &mut self.head
    }

    pub const fn body(&self) -> &B {
        &self.body
    }

    pub const fn body_mut(&mut self) -> &mut B {
        &mut self.body
    }

    pub fn into_parts(self) -> (ResponseHead, B) {
        (self.head, self.body)
    }
}
