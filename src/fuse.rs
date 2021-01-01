//! Extension trait to simplify optionally polling futures.

use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Construct a fusing adapter that is capable of polling an interior future.
///
/// Fuse the future completes, the adapter will switch to an empty state and
/// return [Poll::Pending] until [set][Fuse::set] again.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use tokio::time;
///
/// # #[tokio::main]
/// # async fn main() {
/// let mut sleep = async_fuse::fuse(time::sleep(Duration::from_millis(200)));
/// tokio::pin!(sleep);
///
/// tokio::select! {
///     _ = &mut sleep => {
///         assert!(sleep.is_empty());
///         sleep.set(async_fuse::fuse(time::sleep(Duration::from_millis(200))));
///     }
/// }
///
/// assert!(!sleep.is_empty());
/// # }
/// ```
pub fn fuse<T>(value: T) -> Fuse<T>
where
    T: Future,
{
    Fuse { value: Some(value) }
}

/// Construct an empty fuse.
///
/// # Examples
///
/// ```rust
/// use tokio::time;
///
/// # #[tokio::main]
/// # async fn main() {
/// let mut sleep = async_fuse::empty::<time::Sleep>();
/// tokio::pin!(sleep);
///
/// assert!(sleep.is_empty());
/// # }
/// ```
pub fn empty<T>() -> Fuse<T> {
    Fuse { value: None }
}

pin_project! {
    /// Fusing adapter that is capable of polling an interior value that is
    /// being fused using a custom polling function.
    ///
    /// See [fuse] for details.
    pub struct Fuse<T> {
        #[pin]
        value: Option<T>,
    }
}

impl<T> Future for Fuse<T>
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

impl<T> Fuse<T> {
    /// Construct an empty value that can never complete.
    pub fn empty() -> Fuse<T> {
        Fuse { value: None }
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
    /// let mut sleep = async_fuse::fuse(time::sleep(Duration::from_millis(200)));
    /// tokio::pin!(sleep);
    ///
    /// assert!(!sleep.is_empty());
    /// sleep.set(async_fuse::Fuse::empty());
    /// assert!(sleep.is_empty());
    /// # }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }
}

impl<T> Default for Fuse<T> {
    fn default() -> Self {
        Self { value: None }
    }
}
