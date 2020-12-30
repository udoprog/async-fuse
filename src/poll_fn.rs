//! Extension trait to simplify optionally polling futures.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Construct a fusing adapter that is capable of polling an interior value that
/// is being polled using a custom function.
///
/// The value of this container *will not* be cleared, since a common use case
/// is to optionally interact with stream-like things like [Interval] (see below
/// for example).
///
/// For simplicity's sake, this adapter also pins the value it's being
/// constructed with.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use tokio::time;
///
/// # #[tokio::main]
/// # async fn main() {
/// let mut interval = async_fuse::poll_fn(time::interval(Duration::from_millis(200)), time::Interval::poll_tick);
///
/// tokio::select! {
///     _ = &mut interval => {
///         interval.clear();
///     }
/// }
///
/// assert!(interval.is_empty());
/// # }
/// ```
///
/// [Interval]: https://docs.rs/tokio/1/tokio/time/struct.Interval.html
pub fn poll_fn<T, P, O>(value: T, poll: P) -> PollFn<T, P, O>
where
    T: Unpin,
    P: Unpin,
    P: FnMut(&mut T, &mut Context<'_>) -> Poll<O>,
{
    PollFn {
        value: Some(value),
        poll,
    }
}

/// Fusing adapter that is capable of polling an interior value that is
/// being fused using a custom polling function.
///
/// See [poll_fn] for details.
pub struct PollFn<T, P, O>
where
    T: Unpin,
    P: Unpin,
    P: FnMut(&mut T, &mut Context<'_>) -> Poll<O>,
{
    value: Option<T>,
    poll: P,
}

impl<T, P, O> Future for PollFn<T, P, O>
where
    T: Unpin,
    P: Unpin,
    P: FnMut(&mut T, &mut Context<'_>) -> Poll<O>,
{
    type Output = O;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self.as_mut();

        let inner = match this.value.as_mut() {
            Some(inner) => inner,
            None => return Poll::Pending,
        };

        let value = match (this.poll)(inner, cx) {
            Poll::Ready(value) => value,
            Poll::Pending => return Poll::Pending,
        };

        Poll::Ready(value)
    }
}

impl<T, P, O> PollFn<T, P, O>
where
    T: Unpin,
    P: Unpin,
    P: FnMut(&mut T, &mut Context<'_>) -> Poll<O>,
{
    /// Set the fused value to be something else. The previous value will be
    /// dropped.
    ///
    /// The signature of this function is optimized towards being pinned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio::time;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut interval = async_fuse::poll_fn(time::interval(Duration::from_millis(200)), time::Interval::poll_tick);
    ///
    /// interval.set(time::interval(Duration::from_secs(10)));
    /// # }
    /// ```
    pub fn set(&mut self, value: T) {
        self.value = Some(value);
    }

    /// Clear the fused value.
    ///
    /// This will cause the old value to be dropped if present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio::time;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut interval = async_fuse::poll_fn(time::interval(Duration::from_millis(200)), time::Interval::poll_tick);
    ///
    /// assert!(!interval.is_empty());
    /// interval.clear();
    /// assert!(interval.is_empty());
    /// # }
    /// ```
    pub fn clear(&mut self) {
        self.value = None;
    }

    /// Test if the polled for value is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio::time;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut interval = async_fuse::poll_fn(time::interval(Duration::from_millis(200)), time::Interval::poll_tick);
    ///
    /// assert!(!interval.is_empty());
    /// interval.clear();
    /// assert!(interval.is_empty());
    /// # }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }
}
