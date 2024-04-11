- the watchexec library watches files for changes, and notifies the server
- client and server communicate over a simple WebSocket connection
    - [fastwebsockets](https://crates.io/crates/fastwebsockets)

All of these are *already* `async`. So then the question is exactly how to connect the pieces!

1. Get watching working.
2. Get WebSocket talking.
    - Define a `Resource` for each item, based on its fully-resolved URL
    - Notify the client with a message that includes the resource URL in its payload.
3. On client, when receiving the message, find any reference to it, and replace it (with a cache-busting “just make up a hash” kind of thing). That’s a “dumb” approach but… it will work for my purposes.

Note that (3) is a bonus; a working-but-interesting example here might be as simple as “don’t reload *all* files when *any* file changes, only reload *the file which changed*.”

Idea: instead of using `watchexec`, use `notify` and *show* how to wrap it in `async`, so people understand that it is possible to bridge both directions (blocking *on* async, and turning a blocking call *into* async)?

A bit of exploration on that suggests: it might actually be a *very* good hook. The `notify-debouncer-(full|mini)` libraries really don’t do a particularly good job of *actually* debouncing things, so `watchexec` doesn’t even bother using that mechanic. Instead, it implements its own throttling. And throttling is a *great* way to show another piece of async: timeouts. You end up with something which (a) needs to bridge sync-into-`async` code, (b) needs to manage that with time and timeouts, and (c) needs to forward that information over to a *different* asynchronous process.

If anything, it might be… too much code?