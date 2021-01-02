//! Extension trait to simplify optionally polling futures.

use crate::poll;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A fusing adapter that stores a pinned value on the heap.
///
/// For most operations except [poll_inner], if the value completes, the
/// adapter will switch to an empty state and return [Poll::Pending] until
/// [set][Heap::set] again.
///
/// See [Heap::new] for more details.
pub struct Heap<T: ?Sized> {
    value: Option<Pin<Box<T>>>,
}

impl<T> Heap<T> {
    /// Construct a fusing adapter that stores a pinned value on the heap.
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
    pub fn new(value: T) -> Self {
        Heap {
            value: Some(Box::pin(value)),
        }
    }

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
}

impl<T: ?Sized> Heap<T> {
    /// Construct a fusing adapter that stores a pinned value on the heap. This
    /// variant of the constructor takes unsized types by first insisting that
    /// they go through a `Box<T>`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::future::Future;
    /// use tokio::time;
    ///
    /// async fn foo() -> u32 { 1 }
    /// async fn bar() -> u32 { 2 }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut fut = async_fuse::Heap::<dyn Future<Output = u32>>::new_unsized(Box::new(foo()));
    /// let mut total = 0;
    ///
    /// while !fut.is_empty() {
    ///     let value = (&mut fut).await;
    ///
    ///     if value == 1 {
    ///         fut.set_unsized(Box::new(bar()));
    ///     }
    ///
    ///     total += value;
    /// }
    ///
    /// assert_eq!(total, 3);
    /// # }
    /// ```
    pub fn new_unsized(value: Box<T>) -> Self {
        Heap {
            value: Some(value.into()),
        }
    }

    /// Set the value from a box.
    ///
    /// This allows for setting unsized types, such as trait objects.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::future::Future;
    /// use tokio::time;
    ///
    /// async fn foo() -> u32 { 1 }
    /// async fn bar() -> u32 { 2 }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut fut = async_fuse::Heap::<dyn Future<Output = u32>>::empty();
    /// assert!(fut.is_empty());
    /// fut.set_unsized(Box::new(foo()));
    /// assert!(!fut.is_empty());
    /// fut.set_unsized(Box::new(bar()));
    /// assert!(!fut.is_empty());
    /// # }
    /// ```
    pub fn set_unsized(&mut self, value: Box<T>) {
        self.value = Some(value.into());
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

    /// Poll the current value with the given polling implementation.
    ///
    /// This can be used for types which only provides a polling function.
    ///
    /// This will never empty the underlying value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio::sync::mpsc;
    /// use std::future::Future;
    ///
    /// async fn op(n: u32) -> u32 {
    ///     n
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut op1 = async_fuse::Heap::new(op(1));
    ///
    /// assert_eq!(op1.poll_inner(|mut i, cx| i.poll(cx)).await, 1);
    /// assert!(!op1.is_empty());
    ///
    /// op1.set(op(2));
    /// assert_eq!(op1.poll_inner(|mut i, cx| i.poll(cx)).await, 2);
    /// assert!(!op1.is_empty());
    /// # }
    /// ```
    pub async fn poll_inner<P, O>(&mut self, poll: P) -> O
    where
        P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
    {
        poll::PollInner::new(ProjectHeap(self), poll).await
    }

    /// Poll the current value with the given polling implementation.
    ///
    /// This can be used for types which only provides a polling function.
    ///
    /// Once the underlying poll impl returns `Poll::Ready`, the underlying
    /// value will be emptied.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio::sync::mpsc;
    /// use std::future::Future;
    ///
    /// async fn op(n: u32) -> u32 {
    ///     n
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut op1 = async_fuse::Heap::new(op(1));
    ///
    /// assert_eq!(op1.poll_future(|mut i, cx| i.poll(cx)).await, 1);
    /// assert!(op1.is_empty());
    ///
    /// op1.set(op(2));
    /// assert!(!op1.is_empty());
    /// assert_eq!(op1.poll_future(|mut i, cx| i.poll(cx)).await, 2);
    /// assert!(op1.is_empty());
    /// # }
    /// ```
    pub async fn poll_future<P, O>(&mut self, poll: P) -> O
    where
        P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
    {
        poll::PollFuture::new(ProjectHeap(self), poll).await
    }

    /// Poll the current value with the given polling implementation.
    ///
    /// This can be used for types which only provides a polling function, or
    /// types which can be polled multiple streams. Like streams which do not
    /// provide a Stream implementation.
    ///
    /// Will empty the fused value once the underlying poll returns
    /// `Poll::Ready(None)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio::sync::mpsc;
    /// use std::future::Future;
    /// use futures_core::Stream;
    ///
    /// fn op(n: u32) -> impl Stream<Item = u32> {
    ///     async_stream::stream! {
    ///         yield n;
    ///         yield n + 1;
    ///     }
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut op1 = async_fuse::Heap::new(op(1));
    ///
    /// assert!(!op1.is_empty());
    /// assert_eq!(op1.poll_stream(|mut i, cx| i.poll_next(cx)).await, Some(1));
    /// assert_eq!(op1.poll_stream(|mut i, cx| i.poll_next(cx)).await, Some(2));
    /// assert!(!op1.is_empty());
    /// assert_eq!(op1.poll_stream(|mut i, cx| i.poll_next(cx)).await, None);
    /// assert!(op1.is_empty());
    /// # }
    /// ```
    pub async fn poll_stream<P, O>(&mut self, poll: P) -> Option<O>
    where
        P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<Option<O>>,
    {
        poll::PollStream::new(ProjectHeap(self), poll).await
    }

    /// Poll the next value in the stream where the underlying value is unpin.
    ///
    /// Behaves the same as [poll_stream], except that it only works for values
    /// which are [Unpin].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio::sync::mpsc;
    /// use std::future::Future;
    /// use futures_core::Stream;
    ///
    /// fn op(n: u32) -> impl Stream<Item = u32> {
    ///     async_stream::stream! {
    ///         yield n;
    ///         yield n + 1;
    ///     }
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut stream = async_fuse::Heap::new(op(1));
    /// assert!(!stream.is_empty());
    ///
    /// assert_eq!(stream.next().await, Some(1));
    /// assert_eq!(stream.next().await, Some(2));
    /// assert_eq!(stream.next().await, None);
    ///
    /// assert!(stream.is_empty());
    /// # }
    /// ```
    #[cfg(feature = "stream")]
    #[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
    pub async fn next(&mut self) -> Option<T::Item>
    where
        Self: Unpin,
        T: futures_core::Stream,
    {
        self.poll_stream(futures_core::Stream::poll_next).await
    }
}

impl<T> Future for Heap<T>
where
    T: ?Sized + Future,
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

#[cfg(feature = "stream")]
#[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
impl<T> futures_core::Stream for Heap<T>
where
    T: futures_core::Stream,
{
    type Item = T::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner = match &mut self.value {
            Some(inner) => inner.as_mut(),
            None => return Poll::Pending,
        };

        let value = match inner.poll_next(cx) {
            Poll::Ready(value) => value,
            Poll::Pending => return Poll::Pending,
        };

        if value.is_none() {
            self.value = None;
        }

        Poll::Ready(value)
    }
}

impl<T> From<Option<T>> for Heap<T> {
    fn from(value: Option<T>) -> Self {
        Heap {
            value: value.map(Box::pin),
        }
    }
}

impl<T> From<Box<T>> for Heap<T> {
    fn from(value: Box<T>) -> Self {
        Heap {
            value: Some(value.into()),
        }
    }
}

impl<T> From<Option<Box<T>>> for Heap<T> {
    fn from(value: Option<Box<T>>) -> Self {
        Heap {
            value: value.map(Into::into),
        }
    }
}

impl<T: ?Sized> Default for Heap<T> {
    fn default() -> Self {
        Self { value: None }
    }
}

struct ProjectHeap<'a, T: ?Sized>(&'a mut Heap<T>);

impl<'a, T: ?Sized> poll::Project for ProjectHeap<'a, T> {
    type Value = T;

    fn clear(&mut self) {
        self.0.value = None;
    }

    fn project(&mut self) -> Option<Pin<&mut Self::Value>> {
        match &mut self.0.value {
            Some(value) => Some(value.as_mut()),
            None => None,
        }
    }
}
