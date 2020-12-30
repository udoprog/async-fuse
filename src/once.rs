//! Extension trait to simplify optionally polling futures.

use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Construct a fusing adapter that is capable of polling an interior future.
///
/// Once the future completes, the adapter will switch to an empty state and
/// return [Poll::Pending] until [set][Once::set] again.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use tokio::time;
///
/// # #[tokio::main]
/// # async fn main() {
/// let mut sleep = async_fuse::once(time::sleep(Duration::from_millis(200)));
/// tokio::pin!(sleep);
///
/// tokio::select! {
///     _ = &mut sleep => {
///         assert!(sleep.is_empty());
///         sleep.set(async_fuse::once(time::sleep(Duration::from_millis(200))));
///     }
/// }
///
/// assert!(!sleep.is_empty());
/// # }
/// ```
pub fn once<T>(value: T) -> Once<T>
where
    T: Future,
{
    Once { value: Some(value) }
}

pin_project! {
    /// Fusing adapter that is capable of polling an interior value that is
    /// being fused using a custom polling function.
    ///
    /// See [once] for details.
    pub struct Once<T> {
        #[pin]
        value: Option<T>,
    }
}

impl<T> Future for Once<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = match self.as_mut().project().value.as_pin_mut() {
            Some(inner) => inner,
            None => return Poll::Pending,
        };

        let value = match inner.poll(cx) {
            Poll::Ready(value) => value,
            Poll::Pending => return Poll::Pending,
        };

        self.as_mut().project().value.set(None);
        Poll::Ready(value)
    }
}

impl<T> Once<T> {
    /// Construct an empty value that can never complete.
    pub fn empty() -> Once<T> {
        Once { value: None }
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
    /// let mut sleep = async_fuse::once(time::sleep(Duration::from_millis(200)));
    /// tokio::pin!(sleep);
    ///
    /// assert!(!sleep.is_empty());
    /// sleep.set(async_fuse::Once::empty());
    /// assert!(sleep.is_empty());
    /// # }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }
}
