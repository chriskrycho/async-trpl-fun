Conceptual machinery!

> [!warning] All sorts of “ordering” issues here
> This is *not* the right order for this material, I think… but it *is* a useful starting point.

> [!warning] Half outline, half content
> I got on a bit of a roll part-way through this. The first part is bullet-point-tastic, while the the second part is first-draft-tastic instead.

- **Motivation for the features.** Lots of things in software by nature take non-trivial amounts of time. Our options are, roughly:
    - Just wait for them to finish (“blocking”). This is what a synchronous program does.
    - Find some way to write code that does something only once the operation has completed, but *without* blocking. This is what an asynchronous program does.
        - Registered event handlers on some kind of event bus work extremely well and have been used for systems like this (especially UIs) for decades, but:
            - They allow many “sources of truth” for interpreting an event, unless you force a one-responder-per-event rule.
            - They do not usually have to be registered anywhere close to where the event is defined or triggered.

            In combination, those two factors can make it *very* difficult to understand a system, since the logic can live anywhere.

        - Callbacks work reasonably well, but often result in a “pyramid of doom”, and are distinctly non-linear.

        - “Futures” (also named things like “Task” and “Promise” in other languages) allow us to express the idea of an asynchronous operation *as data*. This allows us to apply other normal tools to it.
            - We can pass them around as arguments and return values, just like other data structures.
            - We can write functions/methods which work with them, operating on them in ways that *resemble* callbacks, but where we can use “combinators” or “adapters” as chains of operations on them instead, which eliminates a lot of the nesting and provides a visual structure that more closely matches the behavior of the system.
                - This should feel familiar: it is just like how we treat types like `Option` and `Result` and `Iterator`. (This starts to touch on [[Book/2 Rust Specifics|2 Rust Specifics]], but I am thinking of it here so writing it down here!)
                - And just like those, as we use them a bunch over time, we might find that it is useful to have dedicated syntax. `Result` and `Option` have `?` sugar, `Iterator` has `for` loops, and `Future` has `async` and `.await`. In each case, it is still possible to work with them directly as objects: `Option::map()`, `Iterator::fold()`, and… `FutureExt::flatten()`. (Ellipsis because oh dear: we really need to standardize a bunch of these methods.)

- **Inherent complexity of the problem space.** We cannot *eliminate* the complexity of asynchronous behavior. At the end of the day, we still have to write code which has to deal with the fact that things are going to happen in an unpredictable order in time, and is robust with that. This is hard! However, we *can* tame some of it by eliminating the parts which are more about “code structure” (events, callbacks, etc.) by introducing language capabilities for expressing the asynchronous patterns more directly and “linearly”.

Given that many things take time… could we do something else while waiting around? Well, it depends: what kind of thing are we waiting for? If we are waiting because every core on our machine is fully occupied , then we really cannot do anything in the meantime. We just have to wait until one of those cores is free before we can do anything else. On the other hand, if we are waiting on a network call to return, or a database to respond, or a buffer to be filled up after we started reading a file, then we actually *can* do something in the meantime.

This is what people mean when they talk about operations being I/O bound vs. CPU-bound. Operations which are mostly waiting around for the file system or a database or a network call are waiting for input and output (I/O) operations to finish, and that often leaves CPU resources available to do other kinds of work. For example, if you have a server handling WebSocket connections for a chat application, most of those sockets will be doing nothing all the time, leaving the server free to handle the connections which *are* active and to do other work as well. By contrast, operations like encoding a video or audio file, which can often use all of the CPU (or GPU) grunt that a computer has, do not leave other CPU resources available.

While I/O is one very common case where it is useful to make the code asynchronous, there are other interesting cases, too. For example, even in a case where we know the work *will* saturate all of the computer’s, it might still be handy to switch back and forth between tasks to allow different operations to make progress rather than waiting for all of them to complete in sequence. Every modern desktop and mobile operating system does this, so we usually take it for granted. Modern operating systems usually use preemptive multitasking to allow many more programs to run than we have cores on those machines, without the programs themselves having to be written to explicitly hand over control.

However, when we are writing our own programs, we often know more than an operating system can about the details of what we are doing, which can let us write code which delivers higher performance, nicer user interactions, or both. Async code can be a powerful tool for doing just that, because—particularly as implemented in Rust—it implements a form of *cooperative* multitasking. Unlike preemptive multitasking, in cooperative multitasking the tasks themselves have to indicate that they are at a good stopping point and can safely be paused and resumed later. It turns out that the same mechanics which allow us to treat a network operation as a piece of data—a Future—can also be used to chunk up large or long-running operations. This lets us report progress to the user, for example. That is a form of output, but here the interesting bit is not that we can do things while waiting on output to finish, but rather that we can show output while waiting on a long-running CPU-bound task to finish.

This can also have performance benefits. Chunking up our long-running operations this way can make it so an expensive handful of operations do not block every other operation from making progress. We can therefore sometimes have *more* throughput on our system by allowing operations to make progress concurrently than if we always just run them serially.

==NOTE: some visuals here might help? Also a good place to reference earlier material in the book.==
