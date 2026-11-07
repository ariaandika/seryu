use core::error;

use genos::fd::AsFd;
use seryu::event::EventData;

pub trait Reactor {
    type Error: error::Error;

    fn add<Fd: AsFd, D: EventData>(&self, fd: &Fd, data: D) -> Result<(), Self::Error>;

    fn delete<Fd: AsFd>(&self, fd: &Fd) -> Result<(), Self::Error>;
}
