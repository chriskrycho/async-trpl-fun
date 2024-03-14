## Notes

- [[_index#Overview|Overview]]
- [[Book notes]]
- [[live reload server]]
- [[Ecosystem/_index|Ecosystem]]:
    - [[Ecosystem/Tokio|Tokio]]
    - [[Ecosystem/smol|smol]]
    - [[Ecosystem/async_std|async_std]]
    - [[Ecosystem/futures-rs|futures-rs]]
- Other interesting crates/projects using async
    - [[lilos]]


## Overview

### Mental model

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

Cliff Biffle [asserts](https://cliffle.com/blog/async-inversion/) (I think accurately!) that async in Rust is two key things, brought together in a powerful way:

- “an inversion of control”, where the caller gets control over the flow of the body of the async function
- a way of building state machines which are much less error prone because they are *not* managed by hand

Nick Cameron [writes](https://www.ncameron.org/blog/what-is-an-async-runtime/#:~:text=An%20async%20function%20is%20just%20a%20convenient%20way%20to%20write%20a%20function%20which%20returns%20a%20future.%20All%20that%20is%20required%20is%20for%20something%20to%20call%20poll.):

> An async function is just a convenient way to write a function which returns a future. All that is required is for something to call `poll`.…
> 
> But it turns out that how one calls the `poll` function is an important and difficult question. We don't want to call it too often because that would waste CPU cycles and potentially starve other tasks (we'll get to what exactly a task is soon) from making progress. We don't want to call it too infrequently though because that means our future won't make progress as quickly as it should.
> 
> Furthermore, there is no best way to solve this problem. Different constraints and environments make different approaches more or less optimal.…
>
> Operating systems provide functionality for this kind of asynchronous IO, but it is very low-level (e.g., the epoll API). A good executor must interact with the OS so that it can wake futures at the optimal time (this is sometimes called a reactor).

### Keywords

`.await`, per the `IntoFuture` trait (but, notably, *not* [the Keyword `await` page](https://doc.rust-lang.org/1.76.0/std/keyword.await.html)!):

> The `.await` keyword desugars into a call to `IntoFuture::into_future` first before polling the future to completion. `IntoFuture` is implemented for all `T: Future` which means the `into_future` method will be available on all futures.

This means you can always call `.await` on any type which implements `Future`, but *also* on any type which implements `IntoFuture`. Thus, e.g., [[Ecosystem/Tokio|Tokio]]’s [`JoinHandle`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html) (its implementation of an `async` version of [`std::thread::JoinHandle`](https://doc.rust-lang.org/1.76.0/std/thread/struct.JoinHandle.html)) has an `impl Future`, so you can directly `.await` it as a result of the desuraging.

### Types/libraries

- `std::future::`
    - `Future`
    - `IntoFuture`

- `std::task::`
    - `Context`
    - `Poll`
    - `Waker`

- [[Key types/Pin|std::pin::Pin]] (and `std::marker::Unpin`)

- `futures::`
    - `future::FutureExt`
    - `stream::`
        - `Stream`
        - `StreamExt`

        …with *some* degree of discussion, unless it moves soon enough, about the way it is being stabilized as `AsyncIterator`
### “Under the hood”

- Ultimately, tasks are stored as anonymous types—analogous to the captures for closures. This has implications for what you (implicitly!) store in them.
- Support for `async fn` in trait: uses GATs because `Future`s need to refer to any captures via lifetime. Wheeeee.
- One key constraint in the design—and the source of a lot of the complexity when you poke at *manual* future implementations (as well as runtime implementations) is what you might think of as “bidirectional communication” between executor and future. The executor *polls* the future, but to avoid just doing that continually (and thus blocking on it!) the task *wakes* the future, i.e. *notifies the executor* that the future is ready to be polled again.

### Hazards

Tokio specifically has a fair number of “do this in the right order or things will `panic!()`.” More generally, there are significant caveats ways you can hang yourself:

- [[#Cancellation]] and cleanup/`Drop` (no `AsyncDrop` behavior today!)
- Forgetting to make sure a waker gets called if you return `Poll::Pending`, else the whole task will hang—see [[Key types/Poll|Poll]] for more. (I *saw* this exact behavior when I tried implementing `Future` for the `Delay` type used in the Tokio tutorial before looking at their implementation, in fact![^why-to-type-it-in])
- It’s worth being clear that there are *far* more hazards in general for implementors than for users of runtimes, e.g. `impl Future` needs to handle calls to `Future::poll()` with different `Waker` instances, since you get a different `Waker` from different tasks, i.e. if you have multiple `async` blocks (I *think* that’s a “correct” example of that behavior). But this is, in general, an implementor hazard, since the *vast* majority of both “normal” library code and especially app code will not be `impl Future`-ing, but instead “just” writing `async` blocks and `.await`-ing various futures, and the majority of *those* will also be generated via other `async` blocks, and so on.
- If you `.await` a *reference* rather than a *value*, the Future continues to exist after completion. If you try to `poll()` it *again* (including via `.await`, I think?), it will panic.
    - The same thing goes for [a `Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html), which is an asynchronous trait *akin to* having an `async` version of `Iterator` (thus is approved to be stabilized as `AsyncIterator`, and has a `poll_next()` method is called by `Future::poll()` (at least in today’s world, where `Stream` is not a first-class citizen)
    - You can `futures::stream::StreamExt::fuse()` exactly the same way you can `Iterator::fuse()`, and you will get back `Poll::Ready(None)` in that case—forever.
    - …and it turns out there is also `futures::future::FuturesExt::fuse()`, which does the same so you can guarantee *its* semantics; in that case it (still somewhat strangely, to my mind!) goes back to returning `Poll::Pending` forever. (Why not have a `Poll::Completed`—other than maybe that at this point it is just plain too late?)

[^why-to-type-it-in]: This is one huge reason to do the “just type it in” mechanic I describe in [this blog post](https://v5.chriskrycho.com/journal/you-have-to-type-it-out/): if you use it as a way to try to do things yourself, too, not *merely* copying, you further improve the quality of your learning.

## Runtimes

- [tokio](https://tokio.rs)
- [smol](https://github.com/smol-rs/smol)
- [futures](https://docs.rs/futures/latest/futures/) (this is complicated! `tokio` depends directly on `futures`, while `smol` uses a small 😑 subset of it called `futures-lite`, but `futures::executor` *is a thing*); see also [[Ecosystem/futures-rs]]
- [async-std](https://async.rs) (see also [[Ecosystem/async_std]])

One thing to notice here is that Tokio’s dominance in the space (which is well-earned!) means it is easy to conflate “what Tokio does” with “how `async`/`.await` works”—but those are very much *not* the same things. E.g. *Tokio* supplies `join!` and `select!`, and others might as well, but they aren’t things which are necessarily part of the core language. And `join!` is a particularly interesting example because it is *on track to be stabilized*… but is not yet, and is only available on nightly, and [has no track to stabilization at this point](https://github.com/rust-lang/rust/issues/91642#issuecomment-992773288); while `select!` is not even currently available at all, for related reasons. (==TODO: Is tokio’s `join!` from `futures`? Probably!==)

## Questions

What is the relationship between the [futures](https://docs.rs/futures/latest/futures/) crate and `std::future`? (And why the deuce is that not clearly documented in the docs for both?!?)

- It looks like the core traits were pulled over at 1.36.0, when it was stabilized, but the `futures` crate has a ton of other capabilities in it, e.g. its own executor.
- The answer is [found](https://rust-lang.github.io/async-book/01_getting_started/03_state_of_async_rust.html#language-and-library-support) in the Async book:
> - The most fundamental traits, types and functions, such as the [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) trait are provided by the standard library.
> - The `async/await` syntax is supported directly by the Rust compiler.
> - Many utility types, macros and functions are provided by the [`futures`](https://docs.rs/futures/) crate. They can be used in any async Rust application.

Additionally, the key types which look like they are duplicated in `futures-rs`… are just re-exported from `std::future`.

## Cancellation

Fundamentally, canceling is related to `Drop`, because (as the Tokio tutorial puts it) “cancellation is performed by dropping a future.” However, one thing which stands out to me about cancellation right up front: Intuitively, I *think* `join!` should have basically all the same downsides as JS’ `Promise.prototype.all`. If the task hangs forever, it hangs forever—and any references it has *also* hang forever. That means it will never trigger any `Drop` implementation, because it does not get dropped!

That means that while `Drop` means cancellation, *actually making that happen* seems like it might be non-trivial—at least, without involving something timeouts or some other means of preemption. For example (pulling on the Tokio tutorial again): if you’re using channels to communicate, you need to handle the case where one of them sends its close message (indicating that *it* got dropped), and do the work to drop your own side so it does not “dangle”. It won’t happen automatically!

> [!note]
> It is *not* the same as e.g. `Promise` rejection in JS. Rather, it is as if a JS `Promise` *could not* reject but were always `Promise<Result<T, E>>`, e.g. using [True Myth](https://true-myth.js.org) or some other such library’s `Result` type.

Thinking about this pair of comments from [[Ecosystem/Tokio|Tokio]]’s docs for `JoinHandle`:

> The `&mut JoinHandle<T>` type is cancel safe. If it is used as the event in a `tokio::select!` statement and some other branch completes first, then it is guaranteed that the output of the task is not lost.
> 
> If a `JoinHandle` is dropped, then the task continues running in the background and its return value is lost.

This is an important distinction. The behavior of the task when *dropped* is the same as it is for `std::thread::JoinHandle`, *and* it is safe for cancellation. Cancellation is a distinct concept from `Drop`, *even though `Drop` is the key way to perform cancellation*. Cancellation is sometimes implicit, e.g. the result of joining a couple tasks and accepting the first one to finish (e.g. `tokio::select!(future_a, future_b).

On the one hand, it is to the community’s credit that there is detailed documentation of cancellation safety (e.g. in [the `tokio::select!` documentation](https://docs.rs/tokio/latest/tokio/macro.select.html)). On the other hand, it seems like a fairly obvious footgun! It is also not 100% obvious to me whether “cancellation safety” _per se_ is actually rigorously defined. These seem fairly different, for example (_ibid._):

> The following methods are not cancellation safe and can lead to loss of data:
> 
> - [`tokio::io::AsyncReadExt::read_exact`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html#method.read_exact "method tokio::io::AsyncReadExt::read_exact")
> - [`tokio::io::AsyncReadExt::read_to_end`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html#method.read_to_end "method tokio::io::AsyncReadExt::read_to_end")
> - [`tokio::io::AsyncReadExt::read_to_string`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html#method.read_to_string "method tokio::io::AsyncReadExt::read_to_string")
> - [`tokio::io::AsyncWriteExt::write_all`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWriteExt.html#method.write_all "method tokio::io::AsyncWriteExt::write_all")
> 
> The following methods are not cancellation safe because they use a queue for fairness and cancellation makes you lose your place in the queue:
> 
> - [`tokio::sync::Mutex::lock`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html#method.lock "method tokio::sync::Mutex::lock")
> - [`tokio::sync::RwLock::read`](https://docs.rs/tokio/latest/tokio/sync/struct.RwLock.html#method.read "method tokio::sync::RwLock::read")
> - [`tokio::sync::RwLock::write`](https://docs.rs/tokio/latest/tokio/sync/struct.RwLock.html#method.write "method tokio::sync::RwLock::write")
> - [`tokio::sync::Semaphore::acquire`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html#method.acquire "method tokio::sync::Semaphore::acquire")
> - [`tokio::sync::Notify::notified`](https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html#method.notified "method tokio::sync::Notify::notified")

Tokio’s doc continues:

> Cancellation safety can be defined in the following way: If you have a future that has not yet completed, then it must be a no-op to drop that future and recreate it. This definition is motivated by the situation where a `select!` is used in a loop. Without this guarantee, you would lose your progress when another branch completes and you restart the `select!` by going around the loop.
> 
> Be aware that cancelling something that is not cancellation safe is not necessarily wrong. For example, if you are cancelling a task because the application is shutting down, then you probably don’t care that partially read data is lost.

This is sort of adjacent to idempotency—but not identical, because of the caveat around completion.

### More to read

- [ ] [Cliff Biffle’s lilos doc on the subject](https://github.com/cbiffle/lilos/blob/main/doc/cancellation.adoc)
- [ ] His deeper dive on it: [# Mutex without lock, Queue without push: cancel safety in lilos](https://cliffle.com/blog/lilos-cancel-safety/)


## Misc.

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

## Ownership

The “invisible state machine”/reified type *really* prominent here—perhaps even more than they are with closures.

Since `async { }` produces an anonymous `Future` type, a lot of the same dynamics as with closures appear. The ownership and lifetime dynamics are mostly invisible/implicit, but just like “closure captures” are ultimately a struct, potentially with borrowed data and therefore lifetime management, the same goes for `async` blocks.

The trick is that each `.await` more or less ends up reifying *its own* data structure. It’s *kind of* like if you were translating this:

```rust
async fn get_em() {
    let mut a = vec![1, 2, 3];
    let mut b = String::from("hello");

    a.push(4);
    let mut more_vals = vec![5, 6, 7];
    a.append(&mut more_vals);
    println!("{a:#?}");
    yield_now().await;

    let first = Box::new(move || {
        a.push(4);
        println!("{a:#?}");
    });

    let c = "cool";
    b.push_str(" ");
    b.push_str(c);
    b.push_str(" person");
    println!("{b:#?}");
    yield_now().await;
}
```

…into this:

```rust
fn run() {
    for f in async_get_em() {
        f();
    }
}

fn async_get_em() -> Vec<Box<dyn FnMut()>> {
    let mut a = vec![1, 2, 3];
    let mut b = String::from("hello");

    let first = Box::new(move || {
        a.push(4);
        println!("{a:#?}");
    });

    let c = "cool";

    let second = Box::new(move || {
        b.push_str(" ");
        b.push_str(c);
        b.push_str(" person");
        println!("{b:#?}");
    });

    vec![first, second]
}
```

This isn’t exactly right, of course, but it is suggestive of the relevant mental intuitions in terms of *other* Rust concepts (though in both cases the captures are still invisible).

### The `'static` bound

How long does a task live? Great big 🤷🏻‍♂️ as far as the compiler is concerned for an `async` block in a *direct* sense: it is entirely up to the executor. That means, though, that an executor/runtime can *define* what it needs to be. However, it will often end up being `'static` precisely because: neither can the executor tell when a given task will wrap up. In some ways, this is the whole point, I think?

As the Tokio docs note, though, this means the *type of the future produced by the block* has to be `'static`, which merely means that the task must own all of its data (though also note that ownership may include owning a reference to an `Arc` or similar!), because it needs to be able to live “arbitrarily” long: as long as the task itself exists, which might be as long as it is (a) not explicitly canceled or (b) the program shut down. The only way to guarantee that is with:

- something which is statically guaranteed to be able to be borrowed the lifetime of the program, i.e. something which is explicitly `&'static`
- something which the task itself owns: in that case it will be dropped when the task is

### Why `async move`

For a value captured by a closure, if it is stack-local but you try to push it into heap-allocated data (`Box::new(|| &v)` or likewise with `Arc` etc.), you have to use a `move` closure instead, `Box::new(move || v)`. The same thing goes for an async block!

### `Send` bounds (e.g. in Tokio)

The same thing applies to the types of the functions in use. When you invoke `tokio::task::spawn_on`, you are bound by its constraints on the future it takes. Since Tokio’s `spawn_on` can move tasks across threads, it constrains its argument to be `Future + Send + 'static` (and the same for the future’s `Output` associated type).

## Laziness

- Failing to `.await` will get you in trouble.
- The compiler helps… with a warning. But *only* a warning.
- This is a tradeoff: it is what lets Rust hand a `Future` to *any* executor and let it do its thing in very different ways, and that really matters. What you do with a `no_std` context on some embedded system (_a la_ lilos) might look *very* different from what you want to do with something like Tokio which is intended to support, among other things, large web services on large servers with tons of heap memory and lots of CPU threads.

## Error handling

Happily, the error-handling story is… basically identical with the error-handling story *without* `async`/`.await`. The `?` operator Just Works™ when you return anything with `impl Try`.[^try-impls] Since `async fn` and `.await` desugar into `-> impl Future` returns, all the normal approaches you would take with error handling—using `std::error`, pulling in whatever combination of `anyhow`, `thiserror`, `miette`, etc., hand-writing your own reporter, you name it—will all Just Work. It requires the same degree of care for thinking about it as ever, but not *more* care.

A good example of this from Tokio (but equally applicable elsewhere):

```rust
use tokio::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    // ...
}
```

This will Just Work™. (And Tokio’s `io::Result` is just `std::io::Result` re-exported.) You can do the same thing in a non-Tokio context; e.g. `futures::executor::block_on`:

```rust
use std::io::Result;
use futures::{
    AsyncReadExt,
    executor::block_on,
};

fn main() -> Result<()> {
    let result = block_on(hello());
    println!("{result}");
}

async fn hello() -> &'static str {
    "Hello, world!"
}
```

[^try-impls]: At present that is `Option<T>`, `Result<T, E>`, `Poll<Result<T, E>>`, `Poll<Option<Result<T, E>>>`, and `ControlFlow<B, C>`.