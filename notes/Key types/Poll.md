---
aliases:
  - Poll
  - std::poll::Poll
---
Tokio’s guide notes:

> [!warning]
> When a future returns `Poll::Pending`, it **must** ensure that the waker is signalled at some point. Forgetting to do this results in the task hanging indefinitely.
>
> Forgetting to wake a task after returning `Poll::Pending` is a common source of bugs.
