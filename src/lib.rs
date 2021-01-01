//! [![Documentation](https://docs.rs/async-fuse/badge.svg)](https://docs.rs/async-fuse)
//! [![Crates](https://img.shields.io/crates/v/async-fuse.svg)](https://crates.io/crates/async-fuse)
//! [![Actions Status](https://github.com/udoprog/async-fuse/workflows/Rust/badge.svg)](https://github.com/udoprog/async-fuse/actions)
//!
//! Helpers for fusing asynchronous computations.
//!
//! This is especially useful in combination with optional branches using
//! [tokio::select], where the future being polled isn't necessarily set.
//!
//! A similar structure is provided by futures-rs called [Fuse]. This however
//! lacks some of the flexibility needed to interact with tokio's streaming
//! types like [Interval] since these no longer implement [Stream].
//!
//! # Examples
//!
//! > This is available as the `ticker` example:
//! > ```sh
//! > cargo run --example ticker
//! > ```
//!
//! ```rust
//! use std::time::Duration;
//! use tokio::time;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let sleep = async_fuse::fuse(time::sleep(Duration::from_millis(100)));
//! tokio::pin!(sleep);
//!
//! for _ in 0..20usize {
//!     (&mut sleep).await;
//!     assert!(sleep.is_empty());
//!     sleep.set(async_fuse::fuse(time::sleep(Duration::from_millis(100))))
//! }
//! # }
//! ```
//!
//! [tokio::select]: https://docs.rs/tokio/1/tokio/macro.select.html
//! [Fuse]: https://docs.rs/futures/0/futures/future/struct.Fuse.html
//! [Stream]: https://docs.rs/futures/0/futures/stream/trait.Stream.html
//! [Interval]: https://docs.rs/tokio/1/tokio/time/struct.Interval.html

#![deny(missing_docs)]

mod fuse;

pub use self::fuse::{empty, fuse, Fuse};
