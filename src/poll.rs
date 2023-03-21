//! Various internal poll impls.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) trait Project {
    type Value: ?Sized;

    /// Clear the projected to value.
    fn clear(&mut self);

    /// Project the value.
    fn project(&mut self) -> Option<Pin<&mut Self::Value>>;
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
    pub(crate) fn new(stack: T, poll: P) -> Self {
        Self { stack, poll }
    }

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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (stack, poll) = self.project();

        let value = match stack.project() {
            Some(value) => value,
            None => return Poll::Pending,
        };

        let output = match poll(value, cx) {
            Poll::Ready(output) => output,
            Poll::Pending => return Poll::Pending,
        };

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
    pub(crate) fn new(stack: T, poll: P) -> Self {
        Self { stack, poll }
    }

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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (stack, poll) = self.project();

        let value = match stack.project() {
            Some(value) => value,
            None => return Poll::Pending,
        };

        match poll(value, cx) {
            Poll::Ready(output) => Poll::Ready(output),
            Poll::Pending => Poll::Pending,
        }
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
    pub(crate) fn new(stack: T, poll: P) -> Self {
        Self { stack, poll }
    }

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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (stack, poll) = self.project();

        let value = match stack.project() {
            Some(value) => value,
            None => return Poll::Pending,
        };

        let output = match poll(value, cx) {
            Poll::Ready(output) => output,
            Poll::Pending => return Poll::Pending,
        };

        if output.is_none() {
            stack.clear();
        }

        Poll::Ready(output)
    }
}
