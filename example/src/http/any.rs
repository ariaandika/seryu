use core::pin::Pin;
use core::ptr::{self, NonNull};
use core::task::{Context, Poll};

use seryu::alloc::Allocator;

// ===== AnyFuture =====

#[repr(C)]
pub struct AnyFuture<A: Allocator> {
    ptr: NonNull<AnyFutureRepr<()>>,
    alloc: A,
}

#[repr(C)]
struct AnyFutureRepr<T = ()> {
    drop: fn(repr: *mut (), alloc: *mut ()),
    poll: fn(repr: *mut (), cx: &mut Context) -> Poll<()>,
    data: T,
}

impl<A: Allocator> Drop for AnyFuture<A> {
    fn drop(&mut self) {
        unsafe {
            let me = self.ptr.as_ptr();
            let drop = (*me).drop;
            drop(me.cast(), &raw mut self.alloc as *mut _);
        }
    }
}

impl<A: Allocator> Future for AnyFuture<A> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            let me = self.get_unchecked_mut().ptr.as_ptr();
            let poll = (*me).poll;
            poll(me.cast(), cx)
        }
    }
}

impl<A: Allocator> AnyFuture<A> {
    pub fn new<F: Future<Output = ()>>(fut: F, alloc: A) -> Result<Self, A::Error> {
        let ptr = alloc.allocate::<AnyFutureRepr<F>>()?;
        unsafe {
            ptr.write(AnyFutureRepr {
                drop: |p, a| {
                    let me = p.cast::<AnyFutureRepr<F>>();
                    ptr::drop_in_place::<F>(&raw mut (*me).data);
                    (*a.cast::<A>()).deallocate::<AnyFutureRepr<F>>(NonNull::new_unchecked(me));
                },
                poll: |p, cx| {
                    let me = p.cast::<AnyFutureRepr<F>>();
                    F::poll(Pin::new_unchecked(&mut (*me).data), cx)
                },
                data: fut,
            })
        };

        Ok(Self { ptr: ptr.cast(), alloc })
    }
}
