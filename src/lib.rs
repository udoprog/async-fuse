//! Extension trait to simplify optionally polling futures.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A free-form fuse variant that supports polling with a custom poller
/// function.
pub struct Fuse<T> {
    value: Option<Pin<Box<T>>>,
}

impl<T> Fuse<T> {
    /// Construct a new fused value.
    pub fn new(value: T) -> Fuse<T> {
        Self {
            value: Some(Box::pin(value)),
        }
    }

    /// Set the fused value to be something else.
    ///
    /// This will cause the old value to be dropped.
    pub fn set(&mut self, value: T) {
        self.value = Some(Box::pin(value));
    }

    /// Clear the fused value.
    ///
    /// This will cause the old value to be dropped if present.
    pub fn clear(&mut self) {
        self.value = None;
    }

    /// Test if the value is empty.
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }

    /// Try to poll the fused value with the given polling implementation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use std::future::Future;
    /// use async_fuse::Fuse;
    /// use tokio::time;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut sleep = Fuse::new(time::sleep(Duration::from_millis(200)));
    ///
    ///     tokio::select! {
    ///         _ = sleep.poll_fn(time::Sleep::poll) => {
    ///             sleep.clear();
    ///         }
    ///     }
    ///
    ///     assert!(sleep.is_empty());
    /// }
    /// ```
    pub fn poll_fn<P, O>(&mut self, poll: P) -> PollFn<'_, T, P, O>
    where
        P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
    {
        PollFn { fuse: self, poll }
    }
}

/// Adapter future to poll a fused value.
pub struct PollFn<'a, T, P, O>
where
    P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
{
    fuse: &'a mut Fuse<T>,
    poll: P,
}

impl<'a, T, P, O> Future for PollFn<'a, T, P, O>
where
    P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: we're not doing anything with the type to violate pinning
        // guarantees.
        //
        // In particular we're taking care not to:
        // * Move the fused value (it's only ever being set to `None`).
        let this = unsafe { Pin::into_inner_unchecked(self) };

        let inner = match &mut this.fuse.value {
            Some(inner) => inner,
            None => return Poll::Pending,
        };

        let value = match (this.poll)(inner.as_mut(), cx) {
            Poll::Ready(value) => value,
            Poll::Pending => return Poll::Pending,
        };

        Poll::Ready(value)
    }
}
