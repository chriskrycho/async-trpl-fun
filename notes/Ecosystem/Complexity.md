---
external-link: https://v5.chriskrycho.com/journal/async-rust-complexity/
---
This claim [from the async-std book](https://book.async.rs/concepts) is interesting:

> [Rust Futures](https://en.wikipedia.org/wiki/Futures_and_promises) have the reputation of being hard. We don't think this is the case. They are, in our opinion, one of the easiest concurrency concepts around and have an intuitive explanation.
> 
> However, there are good reasons for that perception. Futures have three concepts at their base that seem to be a constant source of confusion: deferred computation, asynchronicity and independence of execution strategy.

One thing this misses: it is also hard because there are a *lot* of options in the space. “Just use Tokio” is a really good and reasonable default as far as I can tell, but the lack of opinions and clear documentation on what to do *from the Rust project* (as well as the mixed story around maturity/stability from many of these) makes it substantially harder for people to get their heads around.

Here’s a prime example: you cannot do non-blocking I/O without adding *some* library:

- Tokio’s `tokio::fs`
- Smol’s `async-fs` subcrate (`smol::fs`)
- async-std’s `async_std::fs`

This *also* causes significant complexity for people learning: there is no “progressive disclosure of complexity” here. Instead, you *have* to get your head around a bunch of pieces before you can do *anything* meaningful. At a minimum, you have to pick a runtime, and that immediately prompts you to ask: “But what runtime do I pick? What are the differences?” That in turn immediately exposes you to all of the complexity in the space.

A key decision Rust makes in general is not to pretend complexity does not exist, but to try to (a) expose it in reasonable ways and (b) improve on the state of the art. Here, I think Rust is succeeding reasonably well at (a), and in part at (b) in terms of mechanics and implementation—but not *at all* on (b) in terms of the usability.