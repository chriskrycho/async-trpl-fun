---
aliases:
  - async_std
  - async-std
---
> `async-std` is a foundation of portable Rust software, a set of minimal and battle-tested shared abstractions for the [broader Rust ecosystem](https://crates.io/). It offers std types, like [`Future`](https://docs.rs/async-std/latest/async_std/future/trait.Future.html) and [`Stream`](https://docs.rs/async-std/latest/async_std/stream/trait.Stream.html), library-defined [operations on language primitives](https://docs.rs/async-std/latest/async_std/index.html#primitives), [standard macros](https://docs.rs/async-std/latest/async_std/index.html#macros), [I/O](https://docs.rs/async-std/latest/async_std/io/index.html) and [multithreading](https://docs.rs/async-std/latest/async_std/task/index.html), among [many other things](https://docs.rs/async-std/latest/async_std/index.html#what-is-in-the-standard-library-documentation).

- [home page](https://async.rs)
- [repo](https://github.com/async-rs/async-std)
- [book](https://book.async.rs)

## Stability/etc.

- Hasn’t been released in close to two years.
- CI is currently failing. 😬

## Relationship to rest of ecosystem

- supports `futures` crate
- aims to provide same API as `std` itself
- similar in this regard to both [[Ecosystem/smol|smol]] and [[Ecosystem/Tokio|Tokio]]
- shares a bunch of the underlying crates with smol

