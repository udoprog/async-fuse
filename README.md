# async-fuse

[![Documentation](https://docs.rs/async-fuse/badge.svg)](https://docs.rs/async-fuse)
[![Crates](https://img.shields.io/crates/v/async-fuse.svg)](https://crates.io/crates/async-fuse)
[![Actions Status](https://github.com/udoprog/async-fuse/workflows/Rust/badge.svg)](https://github.com/udoprog/async-fuse/actions)

Helpers for fusing asynchronous computations.

This is especially useful in combination with optional branches using
[tokio::select], where the future being polled isn't necessarily set.

A similar structure is provided by futures-rs called [`Fuse`]. This however
lacks some of the flexibility needed to interact with tokio's streaming
types like [Interval] since these no longer implement [Stream].

## Examples

> This is available as the `ticker` example:
> ```sh
> cargo run --example ticker
> ```

```rust
use std::time::Duration;
use tokio::time;

let mut interval = async_fuse::poll_fn(
    time::interval(Duration::from_secs(1)),
    time::Interval::poll_tick,
);

let sleep = async_fuse::once(time::sleep(Duration::from_secs(5)));
tokio::pin!(sleep);

for _ in 0..20usize {
    tokio::select! {
        when = &mut interval => {
            println!("tick: {:?}", when);
        }
        _ = &mut sleep => {
            interval.set(time::interval(Duration::from_millis(200)));
        }
    }
}
```

[tokio::select]: https://docs.rs/tokio/1/tokio/macro.select.html
[`Fuse`]: https://docs.rs/futures/0/futures/future/struct.Fuse.html

License: MIT/Apache-2.0
