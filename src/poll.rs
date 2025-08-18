//! Various internal poll impls.

use core::future::Future;
use core::pin::Pin;
use core::task::{ready, Context, Poll};

pub(crate) trait Project {
    type Value: ?Sized;

    /// Clear the projected to value.
    fn clear(&mut self);

    /// Project the value.
    fn project(&mut self) -> Poll<Pin<&mut Self::Value>>;
}

/// Future abstraction created using [`crate::Fuse::poll_future`].
pub(crate) struct PollFuture<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>,
    T: Project,
{
    stack: T,
    poll: P,
}

impl<T, P, O> PollFuture<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>,
    T: Project,
{
    #[inline]
    pub(crate) fn new(stack: T, poll: P) -> Self {
        Self { stack, poll }
    }

    #[inline]
    fn project(self: Pin<&mut Self>) -> (&mut T, &mut P) {
        // Safety: private function and caller doesn't violate pinning
        // guarantees.
        unsafe {
            let this = Pin::get_unchecked_mut(self);
            (&mut this.stack, &mut this.poll)
        }
    }
}

impl<T, P, O> Future for PollFuture<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>,
    T: Project,
{
    type Output = O;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (stack, poll) = self.project();

        let value = ready!(stack.project());
        let output = ready!(poll(value, cx));

        stack.clear();
        Poll::Ready(output)
    }
}

/// Future abstraction created using [`crate::Fuse::poll_inner`].
pub(crate) struct PollInner<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>,
    T: Project,
{
    stack: T,
    poll: P,
}

impl<T, P, O> PollInner<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>,
    T: Project,
{
    #[inline]
    pub(crate) fn new(stack: T, poll: P) -> Self {
        Self { stack, poll }
    }

    #[inline]
    fn project(self: Pin<&mut Self>) -> (&mut T, &mut P) {
        // Safety: private function and caller doesn't violate pinning
        // guarantees.
        unsafe {
            let this = Pin::get_unchecked_mut(self);
            (&mut this.stack, &mut this.poll)
        }
    }
}

impl<T, P, O> Future for PollInner<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>,
    T: Project,
{
    type Output = O;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (stack, poll) = self.project();
        let value = ready!(stack.project());
        poll(value, cx)
    }
}

/// Future abstraction created using [`crate::Fuse::poll_stream`].
pub(crate) struct PollStream<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<Option<O>>,
    T: Project,
{
    stack: T,
    poll: P,
}

impl<T, P, O> PollStream<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<Option<O>>,
    T: Project,
{
    #[inline]
    pub(crate) fn new(stack: T, poll: P) -> Self {
        Self { stack, poll }
    }

    #[inline]
    fn project(self: Pin<&mut Self>) -> (&mut T, &mut P) {
        // Safety: private function and caller doesn't violate pinning
        // guarantees.
        unsafe {
            let this = Pin::get_unchecked_mut(self);
            (&mut this.stack, &mut this.poll)
        }
    }
}

impl<T, P, O> Future for PollStream<T, P, O>
where
    T: Project,
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<Option<O>>,
{
    type Output = Option<O>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (stack, poll) = self.project();

        let value = ready!(stack.project());
        let output = ready!(poll(value, cx));

        if output.is_none() {
            stack.clear();
        }

        Poll::Ready(output)
    }
}
