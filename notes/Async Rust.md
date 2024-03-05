## Core 

What is the Rust equivalent to this?

```ts
function main() {
  hello().then(
    (value) => {
      console.log(value);
    },
    (error) => {
      console.error(error);
    }
  );
}

async function hello(): String {
  return "Hello, world!";
}
```

The short answer is: there is nothing *exactly* equivalent to it, because there is no built-in executor. The main executor people use is Tokio, of course; there are also:

The closest thing in Rust to the above is going to be something like this (with `futures` in `Cargo.toml`):

```rust
fn main() {
    let result = futures::executor::block_on(hello());
    println!("{result}");
}

async fn hello() -> &'static str {
    "Hello, world!"
}
```

It *does* feel weird not to have a “good default” baked in, and given the prominence of `futures`, it is *doubly* weird to me not to just ship that executor… but I can imagine the bike-shedding that proposing as much would produce. 🥴 Basically every executor out there ends up shipping ~`block_on` because its utility is so high, AFAICT.

## Notable runtimes

- [tokio](https://tokio.rs)
- [smol](https://github.com/smol-rs/smol)
- [futures](https://docs.rs/futures/latest/futures/) (this is complicated! `tokio` depends directly on `futures`, while `smol` uses a small 😑 subset of it called `futures-lite`)

## Questions

- What is the relationship between the [futures](https://docs.rs/futures/latest/futures/) crate and `std::future`? (And why the deuce is that not clearly documented in the docs for both?!?)
    - It looks like the core traits were pulled over at 1.36.0, when it was stabilized, but the `futures` crate has a ton of other capabilities in it, e.g. its own executor.
    - The answer is [found](https://rust-lang.github.io/async-book/01_getting_started/03_state_of_async_rust.html#language-and-library-support) in the Async book:
        > - The most fundamental traits, types and functions, such as the [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) trait are provided by the standard library.
        > - The `async/await` syntax is supported directly by the Rust compiler.
        > - Many utility types, macros and functions are provided by the [`futures`](https://docs.rs/futures/) crate. They can be used in any async Rust application.

---

Cliff Biffle [asserts](https://cliffle.com/blog/async-inversion/) (I think accurately!) that async in Rust is two key things, brought together in a powerful way:

- “an inversion of control”, where the caller gets control over the flow of the body of the async function
- a way of building state machines which are much less error prone because they are *not* managed by hand

---

- There is no `Debug` implementation for the anonymous `Future` created by `async fn`, which makes sense: it is not the case that `Future: Debug`, and `Future` is a trait, and `async fn foo() -> T` is *roughly* like `fn foo() -> impl Future<Output = T>`.
- Every `Future` you interact with has a concrete struct or enum backing it. This *should* be obvious but, weirdly, was not obvious to me.
    - Most of the time, you are not interacting directly with them.
    - Analogy for my own brain: this is kind of like `View` in SwiftUI: you *often* create a new `View` implementation which is a concrete `struct` there. It is *not* like `Promise` in JS, which while technically an interface *and* a class, is almost always done via the built-in `Promise` class.
    - How often do you explicitly create `Future`s yourself in Rust?
        - Probably depends on whether you’re an app author or a library author!
- Executors can and do distinguish whether they are multi-threaded or single-threaded.
    - The `futures::executor` module provides `LocalPool` as an explicit control for this, and `block_on` uses it by default, but also provides a `ThreadPool`; and both `LocalPool` and `ThreadPool` provide an implementation of `futures::task::Spawn`.
    - Likewise, `smol`’s `async_executor` ships [`Executor`](https://docs.rs/async-executor/1.8.0/async_executor/struct.Executor.html) and [`LocalExecutor`](https://docs.rs/async-executor/1.8.0/async_executor/struct.LocalExecutor.html) so users can choose.
- `futures` provides equivalents to some key sync APIs from `std`: `AsyncBufRead`, `AsyncRead`, `AsyncSeek`, `AsyncWrite` (cf. [`std::io::BufRead`](), [`std::io::Read`](https://doc.rust-lang.org/1.76.0/std/io/trait.Read.html), [`std::io::Seek`](https://doc.rust-lang.org/1.76.0/std/io/trait.Seek.html), [`std::io::Write`](https://doc.rust-lang.org/1.76.0/std/io/trait.Write.html))

## Related

- [[Tokio notes]]