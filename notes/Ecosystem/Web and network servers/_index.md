---
aliases:
  - Web and network servers
---

[[Ecosystem/Tokio|Tokio]], [[Ecosystem/async_std|async-std]], [[Ecosystem/smol|smol]], etc., are just the *foundation* layers for doing async work, and they are intentionally *not* specialized (at least: not explicitly) to any particular kind of work. Instead, they are designed to be *only* the base runtime layer so you can build other kinds of things on top of them. For example: [watchexec](https://watchexec.github.io) uses Tokio to do its work in an async fashion but has *nothing* to do with the network (unless you have it watch a file descriptor which is a network socket, I suppose!).

Thus, if you want to build network services (e.g. web servers), you need layers which sit *on top of* those runtimes:

- Hyper for the _de facto_ HTTP/1 and HTTP/2 implementations
- Tower for a nice set of shared abstractions for the ecosystem’s middleware needs
- A library which composes together Hyper and Tower and then provides ergonomic niceties on top of them. For example:
    - axum
    - warp

While a lot of these will just show the `#[tokio::main] async fn main() { … }` pattern, it is (as with anything in the async ecosystem!) not actually necessary: you can always use `futures::executor::block_on(async { … })` or the like.