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
