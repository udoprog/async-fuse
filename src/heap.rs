//! Extension trait to simplify optionally polling futures.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Fusing adapter that is capable of polling an interior value that is
/// being fused using a custom polling function.
///
/// See [heap_fuse] for details.
pub struct Heap<T> {
    value: Option<Pin<Box<T>>>,
}

impl<T> Heap<T> {
    /// Construct a fusing adapter that is capable of polling an interior future.
    ///
    /// Heap the future completes, the adapter will switch to an empty state and
    /// return [Poll::Pending] until [set][Heap::set] again.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use tokio::time;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = async_fuse::Heap::new(time::sleep(Duration::from_millis(200)));
    ///
    /// tokio::select! {
    ///     _ = &mut sleep => {
    ///         assert!(sleep.is_empty());
    ///         sleep.set(time::sleep(Duration::from_millis(200)));
    ///     }
    /// }
    ///
    /// assert!(!sleep.is_empty());
    /// # }
    /// ```
    pub fn new(value: T) -> Self
    where
        T: Future,
    {
        Heap {
            value: Some(Box::pin(value)),
        }
    }

    /// Construct an empty heap fuse.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio::time;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = async_fuse::Heap::<time::Sleep>::empty();
    ///
    /// assert!(sleep.is_empty());
    /// # }
    /// ```
    pub fn empty() -> Self {
        Self::default()
    }
}

impl<T> Future for Heap<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = match &mut self.value {
            Some(inner) => inner.as_mut(),
            None => return Poll::Pending,
        };

        let value = match inner.poll(cx) {
            Poll::Ready(value) => value,
            Poll::Pending => return Poll::Pending,
        };

        self.value = None;
        Poll::Ready(value)
    }
}

impl<T> Heap<T> {
    /// Set the fused value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio::time;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = async_fuse::Heap::new(time::sleep(Duration::from_millis(200)));
    ///
    /// assert!(!sleep.is_empty());
    /// sleep.set(time::sleep(Duration::from_millis(200)));
    /// assert!(!sleep.is_empty());
    /// # }
    /// ```
    pub fn set(&mut self, value: T) {
        self.value = Some(Box::pin(value));
    }

    /// Clear the fused value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio::time;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = async_fuse::Heap::new(time::sleep(Duration::from_millis(200)));
    ///
    /// assert!(!sleep.is_empty());
    /// sleep.clear();
    /// assert!(sleep.is_empty());
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
    /// let mut sleep = async_fuse::Heap::new(time::sleep(Duration::from_millis(200)));
    ///
    /// assert!(!sleep.is_empty());
    /// sleep.clear();
    /// assert!(sleep.is_empty());
    /// # }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }
}

impl<T> Default for Heap<T> {
    fn default() -> Self {
        Self { value: None }
    }
}
