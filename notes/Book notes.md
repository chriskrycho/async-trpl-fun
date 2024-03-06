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

We are going to want to lean on the previous discussion of parallelism vs. concurrency in the book, which I *believe* exists around threading. When we reintroduce it is likely the time to use it as a way to distinguish between what threads do well and what tasks do well. Tokio’s explanation here is useful:

> If you alternate between two tasks, then you are working on both tasks concurrently, but not in parallel. For it to qualify as parallel, you would need two people, one dedicated to each task.
> 
> One of the advantages of using Tokio is that asynchronous code allows you to work on many tasks concurrently, without having to work on them in parallel using ordinary threads. In fact, Tokio can run many tasks concurrently on a single thread!

Substitute in “async” for “Tokio” here to get a pretty reasonable summary take.

## I/O bound and CPU bound

We don’t need to hyper-focus on the terminology, but giving people an intuition for the difference is probably important. I think the rough question to prime people’s pumps (as it were) is: “What keeps you from making progress? Is it talking over the network/reading and writing files? Or is it actually just processing the data you got from the network/file system?”

But as [[lilos]] suggests: I/O-vs.-CPU-bound is not the only reason to think about `async`. Having *super* lightweight concurrency is useful in its own right for handling many small tasks… the way an OS does.

## “Green threads”

Is this something we actually need to dig into? It may warrant *mentioning*, at least as an aside, so that people know what it is when it comes up, i.e. “(You will sometimes hear this task-based approach referred to as ‘green threads’.)” I think there is some terminological confusion inherent in “green threads” language, though. The Tokio docs, for example, go straight from distinguishing between concurrency and parallelism *by emphasizing the difference between threading and tasks*… and then immediately start using the language of “green threads” to describe what a Tokio task is:

> A Tokio task is an asynchronous green thread.

The key is that a “green thread”—a “task”—enable concurrency but do *not* enable parallelism. You can *combine* OS-level threads, processes, or other means of parallelism *with* task-based concurrency, though.