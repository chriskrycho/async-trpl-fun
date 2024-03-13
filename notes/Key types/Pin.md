---
aliases:
  - std::pin::Pin
  - Pin
---
## Key bits

- Uses the type system/compiler to prevent moving *values* from being moved; “a Rust compiler considers all types movable” regardless of `Pin`. Prevents getting any `&mut T` access even when wrapping `Pin<&mut T>`.
- Has paired `Unpin` auto trait for types which are movable regardless of being `Pin`’d, e.g. basic types and references.
    - References being movable might be a surprise for a moment, but it’s fine: the key is that a thing *being referenced* not move, not the reference itself.

## Misc.

Related: `Box::pin` and `Box::into_pin`, which are likely common use patterns with `Pin` itself. Note that `Pin::new(Box::new("hey"))` is, under the hood. implementation-identical with `Box::new("hey").into_pin()` and *also* `Box::new("hey").into()` (with `Pin` as the target) since `impl<T> From<Box<T>> for Pin<Box<T>>` uses `Box::into_pin`, and `Pin::new()` and `Box::into_pin()` both just do `unsafe { Pin::new_unchecked(pointer) }`.

## When, though?

This bit from Tokio’s tutorial is (a) totally understandable in context and also (b) hilariously bad in the “oh no what have we done” POV:

> Although we covered `Future` in [the previous chapter](https://tokio.rs/tokio/tutorial/async), this error still isn't very clear. If you hit such an error about `Future` not being implemented when attempting to call `.await` on a **reference**, then the future probably needs to be pinned.

The docs on `tokio::pin!` are helpfully suggestive:

> Calls to `async fn` return anonymous [`Future`](https://doc.rust-lang.org/nightly/core/future/future/trait.Future.html "trait core::future::future::Future") values that are `!Unpin`. These values must be pinned before they can be polled. Calling `.await` will handle this, but consumes the future. If it is required to call `.await` on a `&mut _` reference, the caller is responsible for pinning the future.

This gets at the actual underlying reason: the *signature for `Future::poll()`*:

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
```

That is, if you have a `Future` reference which is *not* `Pin`’d, you cannot call `poll()`.