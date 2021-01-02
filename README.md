# async-fuse

[![Documentation](https://docs.rs/async-fuse/badge.svg)](https://docs.rs/async-fuse)
[![Crates](https://img.shields.io/crates/v/async-fuse.svg)](https://crates.io/crates/async-fuse)
[![Actions Status](https://github.com/udoprog/async-fuse/workflows/Rust/badge.svg)](https://github.com/udoprog/async-fuse/actions)

Helpers for "fusing" asynchronous computations.

A fused operation has a well-defined behavior once the operation has
completed. In this library, it means that a fused operation that has
completed will *block forever* by returning [Poll::Pending].

This is similar to the [Fuse] type provided in futures-rs, but provides more
utility allowing it to interact with types which does not implement
[FusedFuture] or [FusedStream] as is now the case with all core Tokio types
since 1.0.

This is especially useful in combination with optional branches using
[tokio::select], where the future being polled is optionally set. So instead
of requiring a [branch precondition], the future will simply be marked as
pending indefinitely and behave accordingly when polled.

## Features

* `stream` - Makes [Heap] and [Stack] implement the [Stream] trait if they
  contain one.

## Examples

> This is available as the `stack_ticker` example:
> ```sh
> cargo run --example stack_ticker
> ```

```rust
use std::time::Duration;
use tokio::time;

let mut duration = Duration::from_millis(500);

let sleep = async_fuse::Stack::new(time::sleep(duration));
tokio::pin!(sleep);

let update_duration = async_fuse::Stack::new(time::sleep(Duration::from_secs(2)));
tokio::pin!(update_duration);

for _ in 0..10usize {
    tokio::select! {
        _ = &mut sleep => {
            println!("Tick");
            sleep.set(async_fuse::Stack::new(time::sleep(duration)));
        }
        _ = &mut update_duration => {
            println!("Tick faster!");
            duration = Duration::from_millis(250);
        }
    }
}
```

For some types it might be easier to fuse the value on the heap. To make
this easier, we provide the [Heap] type.

As a result, the above example looks pretty similar.


> This is available as the `heap_ticker` example:
> ```sh
> cargo run --example heap_ticker
> ```

```rust
use std::time::Duration;
use tokio::time;

let mut duration = Duration::from_millis(500);

let mut sleep = async_fuse::Heap::new(time::sleep(duration));
let mut update_duration = async_fuse::Heap::new(time::sleep(Duration::from_secs(2)));

for _ in 0..10usize {
    tokio::select! {
        _ = &mut sleep => {
            println!("Tick");
            sleep.set(time::sleep(duration));
        }
        _ = &mut update_duration => {
            println!("Tick faster!");
            duration = Duration::from_millis(250);
        }
    }
}
```

[Fuse]: https://docs.rs/futures/0/futures/future/struct.Fuse.html
[FusedFuture]: https://docs.rs/futures/0/futures/future/trait.FusedFuture.html
[FusedStream]: https://docs.rs/futures/0/futures/stream/trait.FusedStream.html
[Poll::Pending]: https://doc.rust-lang.org/std/task/enum.Poll.html#variant.Pending
[Stream]: https://docs.rs/futures-core/0/futures_core/stream/trait.Stream.html
[Heap]: https://docs.rs/async-fuse/0/async_fuse/struct.Heap.html
[Stack]: https://docs.rs/async-fuse/0/async_fuse/struct.Stack.html
[branch precondition]: https://docs.rs/tokio/1.0.1/tokio/macro.select.html#avoid-racy-if-preconditions
[tokio::select]: https://docs.rs/tokio/1/tokio/macro.select.html

License: MIT/Apache-2.0
