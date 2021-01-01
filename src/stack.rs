//! Extension trait to simplify optionally polling futures.

use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pin_project! {
    /// Fusing adapter for fusing a value on the stack.
    ///
    /// Note that this is used for values which should be pinned as they are
    /// being polled.
    ///
    /// See [Stack::new] for details.
    pub struct Stack<T> {
        #[pin]
        value: Option<T>,
    }
}

impl<T> Future for Stack<T>
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

#[cfg(feature = "stream")]
#[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
impl<T> futures_core::Stream for Stack<T>
where
    T: futures_core::Stream,
{
    type Item = T::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner = match self.as_mut().project().value.as_pin_mut() {
            Some(inner) => inner,
            None => return Poll::Pending,
        };

        let value = match inner.poll_next(cx) {
            Poll::Ready(value) => value,
            Poll::Pending => return Poll::Pending,
        };

        if value.is_none() {
            self.as_mut().project().value.set(None);
        }

        Poll::Ready(value)
    }
}

impl<T> Stack<T> {
    /// Construct a fusing adapter that is capable of polling an interior future.
    ///
    /// Stack the future completes, the adapter will switch to an empty state and
    /// return [Poll::Pending] until [set][Stack::set] again.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use tokio::time;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = async_fuse::Stack::new(time::sleep(Duration::from_millis(200)));
    /// tokio::pin!(sleep);
    ///
    /// tokio::select! {
    ///     _ = &mut sleep => {
    ///         assert!(sleep.is_empty());
    ///         sleep.set(async_fuse::Stack::new(time::sleep(Duration::from_millis(200))));
    ///     }
    /// }
    ///
    /// assert!(!sleep.is_empty());
    /// # }
    /// ```
    pub fn new(value: T) -> Self {
        Self { value: Some(value) }
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
    /// let mut sleep = async_fuse::Stack::<time::Sleep>::empty();
    /// tokio::pin!(sleep);
    ///
    /// assert!(sleep.is_empty());
    /// # }
    /// ```
    pub fn empty() -> Self {
        Stack::default()
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
    /// let mut sleep = async_fuse::Stack::new(time::sleep(Duration::from_millis(200)));
    /// tokio::pin!(sleep);
    ///
    /// assert!(!sleep.is_empty());
    /// sleep.set(async_fuse::Stack::empty());
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
    /// use tokio::time;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sleep = async_fuse::Stack::new(time::sleep(Duration::from_millis(200)));
    /// tokio::pin!(sleep);
    ///
    /// assert!(sleep.as_inner_ref().is_some());
    /// sleep.set(async_fuse::Stack::empty());
    /// assert!(sleep.as_inner_ref().is_none());
    /// # }
    /// ```
    pub fn as_inner_ref(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Convert into its inner mutable value.
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
    /// use tokio::sync::mpsc;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let (tx, rx) = mpsc::unbounded_channel::<u32>();
    /// let mut rx = async_fuse::Stack::new(rx);
    ///
    /// tx.send(42);
    ///
    /// // Manually poll the sleep.
    /// assert_eq!(rx.as_pin_mut().poll_stream(|mut i, cx| i.poll_recv(cx)).await, Some(42));
    ///
    /// rx = async_fuse::Stack::empty();
    /// assert!(rx.is_empty());
    /// # }
    /// ```
    pub fn as_pin_mut(&mut self) -> Pin<&mut Self>
    where
        Self: Unpin,
    {
        Pin::new(self)
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
    /// let op1 = async_fuse::Stack::new(op(1));
    /// tokio::pin!(op1);
    ///
    /// assert_eq!(op1.as_mut().poll_inner(|mut i, cx| i.poll(cx)).await, 1);
    /// assert!(!op1.is_empty());
    ///
    /// op1.set(async_fuse::Stack::new(op(2)));
    /// assert_eq!(op1.as_mut().poll_inner(|mut i, cx| i.poll(cx)).await, 2);
    /// assert!(!op1.is_empty());
    /// # }
    /// ```
    pub async fn poll_inner<P, O>(self: Pin<&mut Self>, poll: P) -> O
    where
        P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
    {
        StackPollInner { stack: self, poll }.await
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
    /// let op1 = async_fuse::Stack::new(op(1));
    /// tokio::pin!(op1);
    ///
    /// assert_eq!(op1.as_mut().poll_future(|mut i, cx| i.poll(cx)).await, 1);
    /// assert!(op1.is_empty());
    ///
    /// op1.set(async_fuse::Stack::new(op(2)));
    /// assert!(!op1.is_empty());
    /// assert_eq!(op1.as_mut().poll_future(|mut i, cx| i.poll(cx)).await, 2);
    /// assert!(op1.is_empty());
    /// # }
    /// ```
    pub async fn poll_future<P, O>(self: Pin<&mut Self>, poll: P) -> O
    where
        P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
    {
        StackPollFuture { stack: self, poll }.await
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
    /// let op1 = async_fuse::Stack::new(op(1));
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
        StackPollStream { stack: self, poll }.await
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
    /// let mut stream = async_fuse::Stack::new(Box::pin(op(1)));
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
        T: futures_core::Stream,
        Self: Unpin,
    {
        self.as_pin_mut()
            .poll_stream(futures_core::Stream::poll_next)
            .await
    }
}

impl<T> From<Option<T>> for Stack<T> {
    fn from(value: Option<T>) -> Self {
        Self { value }
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self { value: None }
    }
}

pin_project! {
    /// Future abstraction created using [Stack::poll_fn].
    struct StackPollFuture<'a, T, P, O> where P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O> {
        stack: Pin<&'a mut Stack<T>>,
        poll: P,
    }
}

impl<'a, T, P, O> Future for StackPollFuture<'a, T, P, O>
where
    P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
{
    type Output = O;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        let value = match this.stack.as_mut().project().value.as_pin_mut() {
            Some(value) => value,
            None => return Poll::Pending,
        };

        let output = match (this.poll)(value, cx) {
            Poll::Ready(output) => output,
            Poll::Pending => return Poll::Pending,
        };

        this.stack.as_mut().project().value.set(None);
        Poll::Ready(output)
    }
}

pin_project! {
    /// Future abstraction created using [Stack::poll_fn].
    struct StackPollInner<'a, T, P, O> where P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O> {
        stack: Pin<&'a mut Stack<T>>,
        poll: P,
    }
}

impl<'a, T, P, O> Future for StackPollInner<'a, T, P, O>
where
    P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<O>,
{
    type Output = O;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        let value = match this.stack.as_mut().project().value.as_pin_mut() {
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
    struct StackPollStream<'a, T, P, O> where P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<Option<O>> {
        stack: Pin<&'a mut Stack<T>>,
        poll: P,
    }
}

impl<'a, T, P, O> Future for StackPollStream<'a, T, P, O>
where
    P: FnMut(Pin<&mut T>, &mut Context<'_>) -> Poll<Option<O>>,
{
    type Output = Option<O>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        let value = match this.stack.as_mut().project().value.as_pin_mut() {
            Some(value) => value,
            None => return Poll::Pending,
        };

        let output = match (this.poll)(value, cx) {
            Poll::Ready(output) => output,
            Poll::Pending => return Poll::Pending,
        };

        if output.is_none() {
            this.stack.as_mut().project().value.set(None);
        }

        Poll::Ready(output)
    }
}
