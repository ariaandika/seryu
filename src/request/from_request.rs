use crate::body::BodyReader;
use crate::request::RequestHead;

pub trait FromRequest<'a>: Sized {
    type Error;

    type Future: Future<Output = Result<Self, Self::Error>>;

    fn from_request(head: &mut RequestHead, body: BodyReader<'a>) -> Self::Future;
}

pub trait FromRequestHead: Sized {
    type Error;

    fn from_request_head(head: &mut RequestHead) -> Result<Self, Self::Error>;
}
