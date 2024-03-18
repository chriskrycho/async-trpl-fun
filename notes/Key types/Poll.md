---
aliases:
  - Poll
  - std::poll::Poll
---
The output of `Future::poll()`. The primitive which `.await` ultimately uses.

Tokio’s guide notes:

> [!warning]
> When a future returns `Poll::Pending`, it **must** ensure that the waker is signalled at some point. Forgetting to do this results in the task hanging indefinitely.
>
> Forgetting to wake a task after returning `Poll::Pending` is a common source of bugs.
