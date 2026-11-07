use core::mem::MaybeUninit;
use core::{mem, ops};

use genos::event::epoll::{Epoll, Event as event, EventType};

// ===== EventData =====

/// Arbitrary event data.
pub trait EventData: Sized {
    fn from_raw(raw: u64) -> Self;

    fn into_raw(self) -> u64;
}

impl EventData for u64 {
    #[inline]
    fn from_raw(data: u64) -> Self {
        data
    }

    #[inline]
    fn into_raw(self) -> u64 {
        self
    }
}

// ===== Interest =====

/// Event interest.
#[derive(Clone, Copy, Default)]
#[repr(transparent)]
pub struct Interest(EventType);

impl Interest {
    pub const READ: Self = Self(Epoll::IN);

    pub const WRITE: Self = Self(Epoll::OUT);

    #[inline]
    pub fn has(&self, other: Interest) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns raw event interest.
    #[inline]
    pub const fn raw(&self) -> EventType {
        self.0
    }
}

macro_rules! impl_bitops {
    ($($name:ident::$ops:ident),*) => {$(
        impl ops::$name for Interest {
            type Output = Self;

            fn $ops(self, rhs: Self) -> Self::Output {
                Self(self.0.$ops(rhs.0))
            }
        }
    )*};
}
impl_bitops!(BitOr::bitor, BitAnd::bitand, BitXor::bitxor);

// ===== Event =====

/// Epoll event.
#[derive(Debug, Default, Clone)]
#[repr(transparent)]
pub struct Event(event);

impl Event {
    pub(super) fn as_raw_slice(slice: &mut [MaybeUninit<Self>]) -> &mut [MaybeUninit<event>] {
        // SAFETY: `Event` is `#[repr(transparent)]` over epoll event
        unsafe { mem::transmute::<&mut [_], &mut [_]>(slice) }
    }

    /// Returns event interest.
    #[inline]
    pub const fn interest(&self) -> Interest {
        Interest(self.0.events)
    }

    /// Returns event data.
    #[inline]
    pub fn data<D: EventData>(&self) -> D {
        D::from_raw(self.0.data)
    }

    /// Returns raw event type.
    #[inline]
    pub const fn raw(&self) -> &event {
        &self.0
    }
}
