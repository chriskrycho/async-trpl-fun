One thing my mucking with a live-reload server highlights (putting it here for convenience) is the need to coordinate across tasks. If, for example, you have distinct tasks for:

- notify/watch (e.g. with `notify-rs` or `watchexec`)
- serve (e.g. with `axum`)
- a WebSocket task for communicating
- some kind of build task

…then you need some ways to coordinate between them:

- So the notify/watch task can inform a build task to do some work.
- So the build task can notify the WebSocket task to send a message to the client.
- So they can all shut down when there is a signal to that effect, e.g. `signal::ctrl_c()` from `tokio_util` (or even in conjunction with watchexec’s signal handling).

Each of those has a slightly different pattern for how to approach it: In the first and second cases, you just need channels for a “message-passing” strategy (presumably, not a *one-shot* because you need to do it repeatedly, in this example). In the final case, the right thing to reach for is a `CancellationToken` (in Tokio land).

> [!note]
> It’s worth figuring out what the corresponding mechanic is in `async-std`/`smol`/etc.: probably getting something like a channel on which you could send a message which you could then use to call `.cancel()` on a task?

Something that was not initially obvious to me was that `select!` (and, for that matter, any other way of structuring it, though `select!`’s magic made it “easier” in some ways) could take a future which was wrapping a *loop*. I am still working to articulate *why* that felt weird: I think it is because it does not map to my intuitions from JavaScript even a little bit. The problem is that in JavaScript, the division between statement and expression makes it so you really cannot have the equivalent of an `async` *block*, and therefore of a fairly trivial inline async *value* that is the result of a `loop`—after all, there *is no value for a loop* in JS. But there *is* in Rust.

```rust
use futures::future::join;
use std::time::Duration;
use tokio::{runtime::Runtime, select, sync::mpsc, time::sleep};

fn main() {
    let rt = Runtime::new().unwrap();
    let (tx, rx) = mpsc::channel(10);
    let h1 = rt.spawn(async move {
        tx.send(String::from("Hi, there; hello!")).await;
    });
    let h2 = rt.spawn(async move {
        process(rx).await;
    });

    rt.block_on(async {
        join(h1, h2).await;
    });
}

async fn process(mut messages: mpsc::Receiver<String>) {
    let example = async {
        loop {
            match messages.recv().await {
                Some(msg) => println!("Got a message! {msg}"),
                None => {
                    // break;
                }
            }
        }
    };

    select! {
        _ = sleep(Duration::from_secs(1)) => {
            println!("Out of time!");
        }
        _ = example => {
            println!("Did the example");
        }
    }
}
```
