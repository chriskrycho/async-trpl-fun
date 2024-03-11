---
aliases:
  - futures-rs
---

- [crate](https://docs.rs/futures/0.3.30/futures/index.html)
- original implementation of `Future` for Rust, but that got moved into `std::future`
- Still supplies all the combinators for working with `Future`s concretely *without* `.await`-ing them.
    - Sort of like `Promise.prototype.(then|catch|finally)`
    - Basically the usual set of combinators you might expect (a) from `Option`, `Result`, and other monad-ish and applicative-ish types (which `Future` is!), or (b) coming from `Task`/`Future`/etc. in other languages.
    - 