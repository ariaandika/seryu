pub use epoll::{Event, EventData, Polling};
pub use app::EventKind;

mod epoll;
mod app;
