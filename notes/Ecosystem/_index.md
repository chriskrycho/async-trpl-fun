---
aliases:
  - Ecosystem
---
Lots of moving parts here. This claim [from the async-std book](https://book.async.rs/concepts) is interesting:

> [Rust Futures](https://en.wikipedia.org/wiki/Futures_and_promises) have the reputation of being hard. We don't think this is the case. They are, in our opinion, one of the easiest concurrency concepts around and have an intuitive explanation.
> 
> However, there are good reasons for that perception. Futures have three concepts at their base that seem to be a constant source of confusion: deferred computation, asynchronicity and independence of execution strategy.

One thing this misses: it is also hard because there are a *lot* of options in the space. “Just use Tokio” is a really good and reasonable default as far as I can tell, but the lack of opinions and clear documentation on what to do *from the Rust project* (as well as the mixed story around maturity/stability from many of these) makes it substantially harder for people to get their heads around.

## Major crates

- [[Ecosystem/async_std|async_std]]
- [[Ecosystem/futures-rs|futures-rs]]
- [[Ecosystem/smol|smol]]
- [[Ecosystem/Tokio|Tokio]]
