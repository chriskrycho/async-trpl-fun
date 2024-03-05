- the watchexec library watches files for changes, and notifies the server
- client and server communicate over a simple WebSocket connection
    - [fastwebsockets](https://crates.io/crates/fastwebsockets)

All of these are *already* `async`. So then the question is exactly how to connect the pieces!