# Option Extensions for Futures

Extension traits for dealing with optional futures and streams.

# Examples

```rust
#![feature(async_await)]

use futures::{future::{self, FusedFuture as _}};
use futures_option::OptionExt as _;

futures::executor::block_on(async {
    let mut f = Some(future::ready::<u32>(1));
    assert!(f.is_some());
    assert_eq!(f.current().await, 1);
    assert!(f.is_none());
    assert!(f.current().is_terminated());
});
```

This is useful when you want to implement optional branches using the
`select!` macro.

```rust
#![feature(async_await)]
#![recursion_limit="128"]

use futures::{future, stream, OptionExt as _, StreamExt as _};

futures::executor::block_on(async {
    let mut value = None;
    let mut values = Some(stream::iter(vec![1u32, 2u32, 4u32].into_iter()).fuse());
    let mut parked = None;

    let mut sum = 0;

    loop {
        futures::select! {
            value = value.current() => {
                sum += value;
                std::mem::swap(&mut parked, &mut values);
            }
            v = values.next() => {
                match v {
                    Some(v) => {
                        value = Some(future::ready(v));
                        std::mem::swap(&mut parked, &mut values);
                    },
                    None => break,
                }
            }
        }
    }

    assert_eq!(7, sum);
});
```