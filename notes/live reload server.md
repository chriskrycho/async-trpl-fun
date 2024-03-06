- the watchexec library watches files for changes, and notifies the server
- client and server communicate over a simple WebSocket connection
    - [fastwebsockets](https://crates.io/crates/fastwebsockets)

All of these are *already* `async`. So then the question is exactly how to connect the pieces!

1. Get watching working.
2. Get WebSocket talking.
    - Define a `Resource` for each item, based on its fully-resolved URL
    - Notify the client with a message that includes the resource URL in its payload.
3. On client, when receiving the message, find any reference to it, and replace it (with a cache-busting “just make up a hash” kind of thing). That’s a “dumb” approach but… it will work for my purposes.
