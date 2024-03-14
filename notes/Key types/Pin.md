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

That is, if you have a `Future` reference which is *not* `Pin`’d, you cannot call `poll()`, because it takes `self` as `Pin<&mut Self>`.

## Locality

You can pin *locally* with `pin!`, but that will “do the right thing” based on context as used with `.await` in general. Using `Pin` directly has to follow all the normal rules around scope and thus stack locals. E.g., from the docs:

```rust
use core::pin::{pin, Pin};

let x: Pin<&mut Foo> = {
    let x: Pin<&mut Foo> = pin!(Foo { /* … */ });
    x
}; // <- Foo is dropped
stuff(x); // Error: use of dropped value
```

In the inner block, `x` is a valid reference to the `Foo` that gets `pin!`’d—but this is kind of shaped like this in practice:

```rust
use core::pin::{pin, Pin};

let x: Pin<&mut Foo> = {
    let mut foo = Foo { /* … */ };
    let x: Pin<&mut Foo> = pin!(&mut foo);
    x
}; // <- foo is dropped
stuff(x); // Error: use of dropped value
```

This is not specific to `Pin`—which helps illustrate the ways that `Pin` is just a normal type overall. If you try to do the same thing with `Box` (or any other “wrapper” type) you get the exact same issue ([playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=67370f843c3d1f79b49c1896c6cb5ef7)):

```rust
fn main() {
    let x: Box<&mut Person> = {
        let mut me = Person {
            name: Some("Chris"),
            age: 36,
        };
        let x = Box::new(&mut me);
        x
    };
    println!("{x:?}");
}

#[derive(Debug)]
struct Person {
    name: Option<&'static str>,
    age: u8,
}
```

The resulting error:

```
error[E0597]: `me` does not live long enough
 --> src/main.rs:7:26
  |
2 |     let x: Box<&mut Person> = {
  |         - borrow later stored here
3 |         let mut me = Person {
  |             ------ binding `me` declared here
...
7 |         let x = Box::new(&mut me);
  |                          ^^^^^^^ borrowed value does not live long enough
8 |         x
9 |     };
  |     - `me` dropped here while still borrowed
```

The errors can be much, *much* worse:

```text
error[E0277]: `from_generator::GenFuture<[static generator@Subscriber::into_stream::{closure#0} for<'r, 's, 't0, 't1, 't2, 't3, 't4, 't5, 't6> {ResumeTy, &'r mut Subscriber, Subscriber, impl Future, (), std::result::Result<Option<Message>, Box<(dyn std::error::Error + Send + Sync + 't0)>>, Box<(dyn std::error::Error + Send + Sync + 't1)>, &'t2 mut async_stream::yielder::Sender<std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't3)>>>, async_stream::yielder::Sender<std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't4)>>>, std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't5)>>, impl Future, Option<Message>, Message}]>` cannot be unpinned
  --> streams/src/main.rs:29:36
   |
29 |     while let Some(msg) = messages.next().await {
   |                                    ^^^^ within `tokio_stream::filter::_::__Origin<'_, impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>`, the trait `Unpin` is not implemented for `from_generator::GenFuture<[static generator@Subscriber::into_stream::{closure#0} for<'r, 's, 't0, 't1, 't2, 't3, 't4, 't5, 't6> {ResumeTy, &'r mut Subscriber, Subscriber, impl Future, (), std::result::Result<Option<Message>, Box<(dyn std::error::Error + Send + Sync + 't0)>>, Box<(dyn std::error::Error + Send + Sync + 't1)>, &'t2 mut async_stream::yielder::Sender<std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't3)>>>, async_stream::yielder::Sender<std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't4)>>>, std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't5)>>, impl Future, Option<Message>, Message}]>`
   |
   = note: required because it appears within the type `impl Future`
   = note: required because it appears within the type `async_stream::async_stream::AsyncStream<std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 'static)>>, impl Future>`
   = note: required because it appears within the type `impl Stream`
   = note: required because it appears within the type `tokio_stream::filter::_::__Origin<'_, impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>`
   = note: required because of the requirements on the impl of `Unpin` for `tokio_stream::filter::Filter<impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>`
   = note: required because it appears within the type `tokio_stream::map::_::__Origin<'_, tokio_stream::filter::Filter<impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>, [closure@streams/src/main.rs:26:14: 26:40]>`
   = note: required because of the requirements on the impl of `Unpin` for `tokio_stream::map::Map<tokio_stream::filter::Filter<impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>, [closure@streams/src/main.rs:26:14: 26:40]>`
   = note: required because it appears within the type `tokio_stream::take::_::__Origin<'_, tokio_stream::map::Map<tokio_stream::filter::Filter<impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>, [closure@streams/src/main.rs:26:14: 26:40]>>`
   = note: required because of the requirements on the impl of `Unpin` for `tokio_stream::take::Take<tokio_stream::map::Map<tokio_stream::filter::Filter<impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>, [closure@streams/src/main.rs:26:14: 26:40]>>`
```