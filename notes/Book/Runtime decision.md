This is not something we have to decide right now, because using the `trpl` crate makes it easy to change our minds and do some refactoring later if we so choose, but: Tokio frequently makes what I think are honestly kind of weird decisions. For example:

Tokio's API choices on `sync::mpsc` are… interesting:

| `std::sync::mpsc` API | `tokio::sync::mpsc` API |
| --------------------- | ----------------------- |
| `sync_channel`        | `channel`               |
| `channel`             | `unbounded_channel`     |

Smol makes a similar move, but with better names:

| `std::sync::mpsc` API | `smol::channel` API |
| --------------------- | ------------------- |
| `sync_channel`        | `bounded`           |
| `channel`             | `unbounded`         |

And there are kind of weird differences in what Tokio’s APIs return, too:

| Library   | `Receiver::recv()` API                                  |
| --------- | ------------------------------------------------------- |
| `std`     | `fn recv(&self) -> Result<T, RecvError>`                |
| Tokio     | `async fn recv(&mut self) -> Option<T>`                 |
| smol      | `async fn recv(&self) -> Result<T, RecvError>`[^smol]   |

Right now, I am papering over these differences for pedagogical simplicity, so that readers are not asking why `unbounded` is now important and can focus on the more important differences between sync and async APIs. And even if we use `smol` instead of `tokio`, we will still need to do *some* of that, as the tables show. Here’s a case where the API decisions run in Tokio’s favor instead:

| Library | Sleep API                 |
| ------- | ------------------------- |
| `std`   | `thread::sleep(Duration)` |
| `tokio` | `time::sleep(Duration)`   |
| `smol`  | `Timer::after(Duration`   |

So in that case I would probably introduce a `trpl::sleep` that just does this:

```rust
use std::time::Duration;
use smol::Timer;

pub async fn sleep(dur: Duration) {
    Timer::after(dur).await;
}
```

Even with some of those quirks, I am mildly inclined to use `smol` for `trpl`? I guess it depends a bit on what we end up choosing to do in Ch. ~~20~~ 21.

But there are also enough of those other differences—like returning `Option<T>` instead of `Future<Output = Result<T, RecvError>>`—that it increasingly feels like it might be worth using `smol`[^async-std] instead. There are *some* divergences there, e.g. `smol` uses `bounded` and `unbounded` instead of `std`’s `channel` and `sync_channel`, but there is overall more consistency with `std`, and fewer places where it just kind of “goes its own way”.

[^smol]: smol’s `recv` actually technically is `fn recv(&self) -> Recv<'a, T>`, but `impl<'a, T> Future for  Recv<T>` with `Output = Result<T, RecvError>`, so it is just the async version of the `std` version. It is typed the way it is purely so that it can have a place to stick the anonymous lifetime, which lets it *not* require everything to be `'static`, which is, you know, kind of nice actually!

[^async-std]: or `async-std`? I feel uncomfortable with its current level of maintenance.