## Structure

The chapter needs to tackle two big things:

- Conceptual machinery: building the mental model for what async is and how it works
- Concrete mechanics: building something small but *real* with async

(That’s just saying “what a programming book should do” in some sense, but it’s helpful for me in thinking explicitly about the two pieces.)

## Misc. observations

- We don’t have an introduction to *what concurrent programming **is*** in the book. It currently just kind of assumes it. Fair enough, but I think we probably *do* need one to make the async stuff make sense.
- The [_Asynchronous Programming in Rust_ book](https://rust-lang.github.io/async-book/) exists! We can lean on that for “here are additional materials to go get a deeper dive.” It is incomplete, but it includes a chapter on building a small executor.
- Two broad approaches: top down (“build something with Tokio/Smol/whatever”) and bottom up (“here are the core types and how they work together”). Can we synthesize the two, bouncing back and forth?
- Length:
    - The web server chapter is ~10,000 words.
    - The scope of `async` is large. At a minimum, it includes—and we probably need to cover, at least to *some* degree—:
        - The basic syntax: `async` blocks (including `async move` blocks), `async fn`, and `.await`
        - `std::future::Future`
        - `std::pin::Pin`
        - `std::task::{Context, Waker, Poll}`
        - some variety of `block_on`, since that’s exposed by basically every executor out there
            - maaaaybe what `block_on` actually does? See comment about the async book above, though.
- The Tokio tutorial is interesting, but I do *not* feel like it actually gives a coherent end-to-end mental model for thinking about async in Rust, despite covering a *lot* of ground. (Part of that, arguably, is because there is no chapter in The Book for it to refer to for that conceptual foundation!)
    - Further: I think this is because their tutorial really *has* to do two separate things: introduce the conceptual machinery for async/await *and* introduce enough of the pieces of Tokio for users to be able to do useful things with it.
    - Also: they (reasonably, but not something we can follow) have a lot of things like `// Some asynchronous logic` as the body of `async fn action() { }`. We are obviously going to need to *not* hand-wave that.

---

We are going to want to lean on the previous discussion of parallelism vs. concurrency in the book, which I *believe* exists around threading. When we reintroduce it is likely the time to use it as a way to distinguish between what threads do well and what tasks do well. Tokio’s explanation here is useful:

> If you alternate between two tasks, then you are working on both tasks concurrently, but not in parallel. For it to qualify as parallel, you would need two people, one dedicated to each task.
> 
> One of the advantages of using Tokio is that asynchronous code allows you to work on many tasks concurrently, without having to work on them in parallel using ordinary threads. In fact, Tokio can run many tasks concurrently on a single thread!

Substitute in “async” for “Tokio” here to get a pretty reasonable summary take.

## Language/library feature status

The elephant in the room is: *wow* is there a lot of stuff that has not shipped, for years and years. 😑 We are going to need to address it and call out (particularly for print) that this is very much a snapshot of how things are *at this point in time*.

If we choose to use Tokio, we are also going to need to be explicit about the reality that Tokio is not—and does not want to be!—the standard, and supplies some things which are likely to end up in stable (via `futures-rs` or otherwise) but also has its own opinions on top of that.

## I/O bound and CPU bound

We don’t need to hyper-focus on the terminology, but giving people an intuition for the difference is probably important. I think the rough question to prime people’s pumps (as it were) is: “What keeps you from making progress? Is it talking over the network/reading and writing files? Or is it actually just processing the data you got from the network/file system?”

But as [[lilos]] suggests: I/O-vs.-CPU-bound is not the only reason to think about `async`. Having *super* lightweight concurrency is useful in its own right for handling many small tasks… the way an OS does.

## “Green threads”

Is this something we actually need to dig into? It may warrant *mentioning*, at least as an aside, so that people know what it is when it comes up, i.e. “(You will sometimes hear this task-based approach referred to as ‘green threads’.)” I think there is some terminological confusion inherent in “green threads” language, though. The Tokio docs, for example, go straight from distinguishing between concurrency and parallelism *by emphasizing the difference between threading and tasks*… and then immediately start using the language of “green threads” to describe what a Tokio task is:

> A Tokio task is an asynchronous green thread.

The key is that a “green thread”—a “task”—enable concurrency but do *not* enable parallelism. You can *combine* OS-level threads, processes, or other means of parallelism *with* task-based concurrency, though.

(There is some question/discussion about whether “green thread” is appropriate as a description of Rust’s async. I don’t think the term matters, but we may need a callout which addresses it.)

## Scope/scale

The [[Ecosystem/Tokio|Tokio]] tutorial, building a mini version of Redis, is (a) very tightly coupled to that specific domain example, and (b) not short! It does have the advantage of showing quite a few parts of the system, though.

- [ ] Figure out how long the Tokio mini-Redis example is in total

Unlike the Tokio tutorial, we should absolutely *not* just resort to skipping/hand-waving it. We need to build whatever we build end to end. The min-redis thing handwaves *most* of it. Of course, how much is “end to end” here? We’re not reimplementing parts of the standard library in general. But more than the level of “[`write_decimal`](https://github.com/tokio-rs/mini-redis/blob/tutorial/src/connection.rs#L225-L238) is implemented by mini-redis” that the Tokio tutorial does a lot of. (It’s *fine* that that’s what Tokio’s tutorial does, just not appropriate for the book.)