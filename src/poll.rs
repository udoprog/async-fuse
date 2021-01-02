//! Various internal poll impls.

use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) trait Project {
    type Value;

    fn project(&mut self) -> Pin<&mut Option<Self::Value>>;
}

pin_project! {
    /// Future abstraction created using [Stack::poll_fn].
    pub(crate) struct PollFuture<T, P, O> where P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>, T: Project {
        stack: T,
        poll: P,
    }
}

impl<T, P, O> PollFuture<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>,
    T: Project,
{
    pub(crate) fn new(stack: T, poll: P) -> Self {
        Self { stack, poll }
    }
}

impl<T, P, O> Future for PollFuture<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>,
    T: Project,
{
    type Output = O;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        let value = match this.stack.project().as_pin_mut() {
            Some(value) => value,
            None => return Poll::Pending,
        };

        let output = match (this.poll)(value, cx) {
            Poll::Ready(output) => output,
            Poll::Pending => return Poll::Pending,
        };

        this.stack.project().set(None);
        Poll::Ready(output)
    }
}

pin_project! {
    /// Future abstraction created using [Stack::poll_fn].
    pub(crate) struct PollInner<T, P, O> where P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>, T: Project {
        stack: T,
        poll: P,
    }
}

impl<T, P, O> PollInner<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>,
    T: Project,
{
    pub(crate) fn new(stack: T, poll: P) -> Self {
        Self { stack, poll }
    }
}

impl<T, P, O> Future for PollInner<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<O>,
    T: Project,
{
    type Output = O;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        let value = match this.stack.project().as_pin_mut() {
            Some(value) => value,
            None => return Poll::Pending,
        };

        let output = match (this.poll)(value, cx) {
            Poll::Ready(output) => output,
            Poll::Pending => return Poll::Pending,
        };

        Poll::Ready(output)
    }
}

pin_project! {
    /// Future abstraction created using [Stack::poll_stream].
    pub(crate) struct PollStream<T, P, O> where P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<Option<O>>, T: Project {
        stack: T,
        poll: P,
    }
}

impl<T, P, O> PollStream<T, P, O>
where
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<Option<O>>,
    T: Project,
{
    pub(crate) fn new(stack: T, poll: P) -> Self {
        Self { stack, poll }
    }
}

impl<T, P, O> Future for PollStream<T, P, O>
where
    T: Project,
    P: FnMut(Pin<&mut T::Value>, &mut Context<'_>) -> Poll<Option<O>>,
{
    type Output = Option<O>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        let value = match this.stack.project().as_pin_mut() {
            Some(value) => value,
            None => return Poll::Pending,
        };

        let output = match (this.poll)(value, cx) {
            Poll::Ready(output) => output,
            Poll::Pending => return Poll::Pending,
        };

        if output.is_none() {
            this.stack.project().set(None);
        }

        Poll::Ready(output)
    }
}
