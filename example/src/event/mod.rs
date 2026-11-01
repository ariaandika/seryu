pub use epoll::{Event, EventData, PollHandle, Polling};
pub use app::EventKind;

mod epoll;
mod app;
