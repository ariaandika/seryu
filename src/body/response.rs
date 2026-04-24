use std::{pin::Pin, task::Poll};
use tcio::bytes::Buf;

pub trait ResponseBody {
    type Data: Buf;

    type Error;

    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>>;

    fn is_end_stream(&self) -> bool;

    fn size_hint(&self) -> (u64, Option<u64>);
}
