//! [![Documentation](https://docs.rs/async-fuse/badge.svg)](https://docs.rs/async-fuse)
//! [![Crates](https://img.shields.io/crates/v/async-fuse.svg)](https://crates.io/crates/async-fuse)
//! [![Actions Status](https://github.com/udoprog/async-fuse/workflows/Rust/badge.svg)](https://github.com/udoprog/async-fuse/actions)
//!
//! Helpers for "fusing" asynchronous computations.
//!
//! A fused operation has a well-defined behavior once the operation has
//! completed. For [Fuse] it means that an operation that has completed will
//! *block forever* by returning [Poll::Pending].
//!
//! This is similar to the [Fuse][futures-fs-fuse] type provided in futures-rs,
//! but provides more utility allowing it to interact with types which does not
//! implement [FusedFuture] or [FusedStream] as is now the case with all Tokio
//! types since 1.0.
//!
//! We also use [Fuse] to represent optional values, just like [Option]. But
//! [Fuse] provides implementations and functions which allow us to safely
//! perform operations over the value when it's pinned. Exactly what's needed to
//! drive a [Stream] (see [next]) or poll a [Future] that might or might not be
//! set.
//!
//! # Features
//!
//! * `stream` - Makes the [Fuse] implement the [Stream] trait if it contains a
//!   stream.
//!
//! # Simplifying [tokio::select]
//!
//! One of the main uses for [Fuse] is to simplify how we use [tokio::select].
//! In this section we'll look at how we can improve an optional branch, where
//! the future being polled might or might not be set.
//!
//! ```rust
//! # #[tokio::main]
//! # async fn main() {
//! let mut maybe_future = Some(async { 42u32 });
//! tokio::pin!(maybe_future);
//!
//! tokio::select! {
//!     value = async { maybe_future.as_mut().as_pin_mut().unwrap().await }, if maybe_future.is_some() => {
//!         maybe_future.set(None);
//!         assert_eq!(value, 42);
//!     }
//!     /* other branches */
//! }
//!
//! assert!(maybe_future.is_none());
//! # }
//! ```
//!
//! The `async` block above is necessary because the future is polled *eagerly*
//! regardless of the [branch precondition]. This would cause the `unwrap` to
//! panic in case the future isn't set. We also need to explicitly set the pin
//! to `None` after completion. Otherwise we might poll it later [which might
//! panic].
//!
//! With [Fuse] we can rewrite the branch and remove the `async` block. It also
//! unsets the future for us after completion.
//!
//! ```rust
//! use async_fuse::Fuse;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let mut maybe_future = Fuse::new(async { 42u32 });
//! tokio::pin!(maybe_future);
//!
//! tokio::select! {
//!     value = &mut maybe_future, if !maybe_future.is_empty() => {
//!         assert_eq!(value, 42);
//!     }
//!     /* other branches */
//! }
//!
//! assert!(maybe_future.is_empty());
//! # }
//! ```
//!
//! Finally if we don't need the [else branch] to evalute we can skip the
//! [branch precondition] entirely. Allowing us to further reduce the code.
//!
//! ```rust
//! use async_fuse::Fuse;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let mut maybe_future = Fuse::new(async { 42u32 });
//! tokio::pin!(maybe_future);
//!
//! tokio::select! {
//!     value = &mut maybe_future => {
//!         assert_eq!(value, 42);
//!     }
//!     /* other branches */
//! }
//!
//! assert!(maybe_future.is_empty());
//! # }
//! ```
//!
//! # Fusing on the stack
//!
//! For the first example we'll be fusing the value *on the stack* using
//! [tokio::pin]. We'll also be updating the fuse as it completes with another
//! sleep with a configurable delay. Mimicking the behavior of [Interval].
//!
//! > This is available as the `stack_ticker` example:
//! > ```sh
//! > cargo run --example stack_ticker
//! > ```
//!
//! ```rust
//! use async_fuse::Fuse;
//! use std::time::Duration;
//! use tokio::time;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let mut duration = Duration::from_millis(500);
//!
//! let sleep = Fuse::new(time::sleep(duration));
//! tokio::pin!(sleep);
//!
//! let update_duration = Fuse::new(time::sleep(Duration::from_secs(2)));
//! tokio::pin!(update_duration);
//!
//! for _ in 0..10usize {
//!     tokio::select! {
//!         _ = &mut sleep => {
//!             println!("Tick");
//!             sleep.set(Fuse::new(time::sleep(duration)));
//!         }
//!         _ = &mut update_duration => {
//!             println!("Tick faster!");
//!             duration = Duration::from_millis(250);
//!         }
//!     }
//! }
//! # }
//! ```
//!
//! # Fusing on the heap
//!
//! For some types it might be easier to fuse the value on the heap. To make
//! this easier, we provide the [Fuse::pin] constructor which provides a fused
//! value which is pinned on the heap.
//!
//! As a result, it looks pretty similar to the above example.
//!
//! > This is available as the `heap_ticker` example:
//! > ```sh
//! > cargo run --example heap_ticker
//! > ```
//!
//! ```rust
//! use async_fuse::Fuse;
//! use std::time::Duration;
//! use tokio::time;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let mut duration = Duration::from_millis(500);
//!
//! let mut sleep = Fuse::pin(time::sleep(duration));
//! let mut update_duration = Fuse::pin(time::sleep(Duration::from_secs(2)));
//!
//! for _ in 0..10usize {
//!     tokio::select! {
//!         _ = &mut sleep => {
//!             println!("Tick");
//!             sleep.set(Box::pin(time::sleep(duration)));
//!         }
//!         _ = &mut update_duration => {
//!             println!("Tick faster!");
//!             duration = Duration::from_millis(250);
//!         }
//!     }
//! }
//! # }
//! ```
//!
//! # Fusing trait objects
//!
//! The following showcases how we can fuse a trait object. Trait objects are
//! useful since they allow the fused value to change between distinct
//! implementations. The price is that we perform dynamic dispatch which has a
//! small cost.
//!
//! Also note that because [CoerceUnsized] is not yet stable, we cannot use
//! [Fuse::pin] for convenience and have to pass a pinned box through
//! [Fuse::new].
//!
//! > This is available as the `trait_object_ticker` example:
//! > ```sh
//! > cargo run --example trait_object_ticker
//! > ```
//!
//! ```rust
//! use async_fuse::Fuse;
//! use std::future::Future;
//! use std::pin::Pin;
//! use std::time::Duration;
//! use tokio::time;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let mut duration = Duration::from_millis(500);
//!
//! let mut sleep: Fuse<Pin<Box<dyn Future<Output = ()>>>> =
//!     Fuse::new(Box::pin(time::sleep(duration)));
//!
//! let mut update_duration: Fuse<Pin<Box<dyn Future<Output = ()>>>> =
//!     Fuse::new(Box::pin(time::sleep(Duration::from_secs(2))));
//!
//! for _ in 0..10usize {
//!     tokio::select! {
//!         _ = &mut sleep => {
//!             println!("Tick");
//!             sleep.set(Box::pin(time::sleep(duration)));
//!         }
//!         _ = &mut update_duration => {
//!             println!("Tick faster!");
//!             duration = Duration::from_millis(250);
//!         }
//!     }
//! }
//! # }
//! ```
//!
//! [branch precondition]: https://docs.rs/tokio/1.0.1/tokio/macro.select.html#avoid-racy-if-preconditions
//! [CoerceUnsized]: https://doc.rust-lang.org/std/ops/trait.CoerceUnsized.html
//! [else branch]: https://docs.rs/tokio/1.0.1/tokio/macro.select.html
//! [Fuse::new]: https://docs.rs/async-fuse/0/async_fuse/struct.Fuse.html#method.new
//! [Fuse::pin]: https://docs.rs/async-fuse/0/async_fuse/struct.Fuse.html#method.pin
//! [Fuse]: https://docs.rs/async-fuse/0/async_fuse/struct.Fuse.html
//! [Fuse]: https://docs.rs/async-fuse/0/async_fuse/struct.Fuse.html
//! [FusedFuture]: https://docs.rs/futures/0/futures/future/trait.FusedFuture.html
//! [FusedStream]: https://docs.rs/futures/0/futures/stream/trait.FusedStream.html
//! [Future]: https://doc.rust-lang.org/std/future/trait.Future.html
//! [futures-fs-fuse]: https://docs.rs/futures/0/futures/future/struct.Fuse.html
//! [Interval]: https://docs.rs/tokio/1/tokio/time/struct.Interval.html
//! [next]: https://docs.rs/async-fuse/0/async_fuse/struct.Fuse.html#method.next
//! [Poll::Pending]: https://doc.rust-lang.org/std/task/enum.Poll.html#variant.Pending
//! [Stream]: https://docs.rs/futures-core/0/futures_core/stream/trait.Stream.html
//! [Stream]: https://docs.rs/futures-core/0/futures_core/stream/trait.Stream.html
//! [tokio::pin]: https://docs.rs/tokio/1/tokio/macro.pin.html
//! [tokio::select]: https://docs.rs/tokio/1/tokio/macro.select.html
//! [which might panic]: https://doc.rust-lang.org/std/future/trait.Future.html#panics

#![deny(missing_docs)]

mod fuse;
mod poll;

pub use self::fuse::Fuse;

#[cfg(feature = "stream")]
pub use futures_core::Stream;
