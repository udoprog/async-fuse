//! Extension trait to simplify optionally polling futures.

use crate::poll;
#[cfg(feature = "stream")]
use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A fusing adapter around a value.
///
/// A `Fuse<T>` is similar to `Option<T>`, with the exception that it provides
/// and API which is more suitable for interacting with asynchronous tasks and
/// pinned values.
///
/// For most polling operations (except [Fuse::poll_inner]), if the value
/// completes, the adapter will switch to an [empty state][Fuse::empty] and
/// return [Poll::Pending]. It can later be updated again with [set][Fuse::set].
///
/// See [Fuse::new] for more details.
pub struct Fuse<T> {
    value: Option<T>,
}

impl<T> Fuse<Pin<Box<T>>> {
    /// Construct a fusing adapter around a value that is already pinned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    /// use std::future::Future;
    /// use tokio::time;
    ///
    /// async fn foo() -> u32 { 1 }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut fut = Fuse::pin(foo());
    /// assert!(!fut.is_empty());
    ///
    /// let value = (&mut fut).await;
    /// assert!(fut.is_empty());
    /// # }
    /// ```
    pub fn pin(value: T) -> Self {
        Self {
            value: Some(Box::pin(value)),
        }
    }
}

impl<T> Fuse<T> {
    /// Construct a fusing adapter around a value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    /// use std::time::Duration;
    /// use tokio::time;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = Fuse::new(time::sleep(Duration::from_millis(200)));
    /// tokio::pin!(sleep);
    ///
    /// tokio::select! {
    ///     _ = &mut sleep => {
    ///         assert!(sleep.is_empty());
    ///         sleep.set(Fuse::new(time::sleep(Duration::from_millis(200))));
    ///     }
    /// }
    ///
    /// assert!(!sleep.is_empty());
    /// # }
    /// ```
    ///
    /// # Example using an unsized trait object
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    /// use std::future::Future;
    /// use std::pin::Pin;
    /// use tokio::time;
    ///
    /// async fn foo() -> u32 { 1 }
    /// async fn bar() -> u32 { 2 }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut fut = Fuse::<Pin<Box<dyn Future<Output = u32>>>>::new(Box::pin(foo()));
    /// let mut total = 0;
    ///
    /// while !fut.is_empty() {
    ///     let value = (&mut fut).await;
    ///
    ///     if value == 1 {
    ///         fut.set(Box::pin(bar()));
    ///     }
    ///
    ///     total += value;
    /// }
    ///
    /// assert_eq!(total, 3);
    /// # }
    /// ```
    pub fn new(value: T) -> Self {
        Self { value: Some(value) }
    }

    /// Set the fused value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    /// use std::time::Duration;
    /// use tokio::time;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = Fuse::new(Box::pin(time::sleep(Duration::from_millis(200))));
    ///
    /// assert!(!sleep.is_empty());
    /// sleep.set(Box::pin(time::sleep(Duration::from_millis(200))));
    /// assert!(!sleep.is_empty());
    /// # }
    /// ```
    ///
    /// # Example setting an unsized trait object
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    /// use std::future::Future;
    /// use std::pin::Pin;
    /// use tokio::time;
    ///
    /// async fn foo() -> u32 { 1 }
    /// async fn bar() -> u32 { 2 }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut fut = Fuse::<Pin<Box<dyn Future<Output = u32>>>>::empty();
    /// assert!(fut.is_empty());
    ///
    /// fut.set(Box::pin(foo()));
    /// assert!(!fut.is_empty());
    ///
    /// fut.set(Box::pin(bar()));
    /// assert!(!fut.is_empty());
    /// # }
    /// ```
    pub fn set(&mut self, value: T)
    where
        Self: Unpin,
    {
        self.value = Some(value);
    }

    /// Clear the fused value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    /// use std::time::Duration;
    /// use tokio::time;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = Fuse::new(Box::pin(time::sleep(Duration::from_millis(200))));
    ///
    /// assert!(!sleep.is_empty());
    /// sleep.clear();
    /// assert!(sleep.is_empty());
    /// # }
    /// ```
    pub fn clear(&mut self)
    where
        Self: Unpin,
    {
        self.value = None;
    }

    /// Construct an empty fuse.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    /// use tokio::time;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = Fuse::<time::Sleep>::empty();
    /// tokio::pin!(sleep);
    ///
    /// assert!(sleep.is_empty());
    /// # }
    /// ```
    pub fn empty() -> Self {
        Fuse::default()
    }

    /// Test if the polled for value is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    /// use std::time::Duration;
    /// use tokio::time;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = Fuse::new(time::sleep(Duration::from_millis(200)));
    /// tokio::pin!(sleep);
    ///
    /// assert!(!sleep.is_empty());
    /// sleep.set(Fuse::empty());
    /// assert!(sleep.is_empty());
    /// # }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }

    /// Access the interior value as a reference.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    /// use std::time::Duration;
    /// use tokio::time;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = Fuse::new(time::sleep(Duration::from_millis(200)));
    /// tokio::pin!(sleep);
    ///
    /// assert!(sleep.as_inner_ref().is_some());
    /// sleep.set(Fuse::empty());
    /// assert!(sleep.as_inner_ref().is_none());
    /// # }
    /// ```
    pub fn as_inner_ref(&self) -> Option<&T> {
        self.value.as_ref()
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
    /// use async_fuse::Fuse;
    /// use std::future::Future;
    /// use tokio::sync::mpsc;
    ///
    /// async fn op(n: u32) -> u32 {
    ///     n
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let op1 = Fuse::new(op(1));
    /// tokio::pin!(op1);
    ///
    /// assert_eq!(op1.as_mut().poll_inner(|mut i, cx| i.poll(cx)).await, 1);
    /// assert!(!op1.is_empty());
    ///
    /// op1.set(Fuse::new(op(2)));
    /// assert_eq!(op1.as_mut().poll_inner(|mut i, cx| i.poll(cx)).await, 2);
    /// assert!(!op1.is_empty());
    /// # }
    /// ```
    pub async fn poll_inner<P, O>(self: Pin<&mut Self>, poll: P) -> O
    where
        P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
    {
        poll::PollInner::new(Project(self), poll).await
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
    /// use async_fuse::Fuse;
    /// use std::future::Future;
    /// use tokio::sync::mpsc;
    ///
    /// async fn op(n: u32) -> u32 {
    ///     n
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let op1 = Fuse::new(op(1));
    /// tokio::pin!(op1);
    ///
    /// assert_eq!(op1.as_mut().poll_future(|mut i, cx| i.poll(cx)).await, 1);
    /// assert!(op1.is_empty());
    ///
    /// op1.set(Fuse::new(op(2)));
    /// assert!(!op1.is_empty());
    /// assert_eq!(op1.as_mut().poll_future(|mut i, cx| i.poll(cx)).await, 2);
    /// assert!(op1.is_empty());
    /// # }
    /// ```
    pub async fn poll_future<P, O>(self: Pin<&mut Self>, poll: P) -> O
    where
        P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
    {
        poll::PollFuture::new(Project(self), poll).await
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
    /// use async_fuse::{Fuse, Stream};
    /// use std::future::Future;
    /// use tokio::sync::mpsc;
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
    /// let op1 = Fuse::new(op(1));
    /// tokio::pin!(op1);
    ///
    /// assert!(!op1.is_empty());
    /// assert_eq!(op1.as_mut().poll_stream(|mut i, cx| i.poll_next(cx)).await, Some(1));
    /// assert_eq!(op1.as_mut().poll_stream(|mut i, cx| i.poll_next(cx)).await, Some(2));
    /// assert!(!op1.is_empty());
    /// assert_eq!(op1.as_mut().poll_stream(|mut i, cx| i.poll_next(cx)).await, None);
    /// assert!(op1.is_empty());
    /// # }
    /// ```
    pub async fn poll_stream<P, O>(self: Pin<&mut Self>, poll: P) -> Option<O>
    where
        P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<Option<O>>,
    {
        poll::PollStream::new(Project(self), poll).await
    }

    /// Access the interior mutable value. This is only available if it
    /// implements [Unpin].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    ///
    /// # fn main() {
    /// let mut rx = Fuse::new(Box::pin(async { 42 }));
    ///
    /// assert!(rx.as_inner_mut().is_some());
    /// # }
    pub fn as_inner_mut(&mut self) -> Option<&mut T>
    where
        Self: Unpin,
    {
        self.value.as_mut()
    }

    /// Helper conversion to a pinned value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use async_fuse::Fuse;
    /// use tokio::sync::mpsc;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let (tx, rx) = mpsc::unbounded_channel::<u32>();
    /// let mut rx = Fuse::new(rx);
    ///
    /// tx.send(42);
    ///
    /// // Manually poll the sleep.
    /// assert_eq!(rx.as_pin_mut().poll_stream(|mut i, cx| i.poll_recv(cx)).await, Some(42));
    ///
    /// rx = Fuse::empty();
    /// assert!(rx.is_empty());
    /// # }
    /// ```
    pub fn as_pin_mut(&mut self) -> Pin<&mut Self>
    where
        Self: Unpin,
    {
        Pin::new(self)
    }

    /// Poll the next value in the stream where the underlying value is unpin.
    ///
    /// Behaves the same as [poll_stream], except that it only works for values
    /// which are [Unpin].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use async_fuse::{Fuse, Stream};
    /// use std::future::Future;
    /// use tokio::sync::mpsc;
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
    /// let mut stream = Fuse::new(Box::pin(op(1)));
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
        T: Stream,
    {
        self.as_pin_mut().poll_stream(Stream::poll_next).await
    }

    fn project(self: Pin<&mut Self>) -> Pin<&mut Option<T>> {
        // Safety: We're projecting into the owned pinned value field, which we
        // otherwise do not move before it's dropped.
        unsafe { Pin::map_unchecked_mut(self, |this| &mut this.value) }
    }
}

impl<T> Future for Fuse<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut poll::PollFuture::new(Project(self), Future::poll)).poll(cx)
    }
}

#[cfg(feature = "stream")]
#[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
impl<T> Stream for Fuse<T>
where
    T: Stream,
{
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut poll::PollStream::new(Project(self), Stream::poll_next)).poll(cx)
    }
}

impl<T> From<Option<T>> for Fuse<T> {
    fn from(value: Option<T>) -> Self {
        Self { value }
    }
}

impl<T> From<Box<T>> for Fuse<Pin<Box<T>>> {
    fn from(value: Box<T>) -> Self {
        Self {
            value: Some(value.into()),
        }
    }
}

impl<T> From<Option<Box<T>>> for Fuse<Pin<Box<T>>> {
    fn from(value: Option<Box<T>>) -> Self {
        Self {
            value: value.map(Into::into),
        }
    }
}

impl<T> Default for Fuse<T> {
    fn default() -> Self {
        Self { value: None }
    }
}

struct Project<'a, T>(Pin<&'a mut Fuse<T>>);

impl<T> poll::Project for Project<'_, T> {
    type Value = T;

    fn clear(&mut self) {
        self.0.as_mut().project().set(None);
    }

    fn project(&mut self) -> Option<Pin<&mut Self::Value>> {
        self.0.as_mut().project().as_pin_mut()
    }
}
