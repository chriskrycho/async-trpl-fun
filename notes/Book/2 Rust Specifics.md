Many languages have these concepts. What is special about or specific to Rust’s implementation?

- Laziness of futures
- Interaction with ownership
- Existence of multiple runtimes suited for different goals and purposes
    - Web servers handling millions of requests a second (Tokio)
    - `no_std` applications which cannot even allocate heap memory (lilos)
- Basics of actual usage could go here, maybe? (We might also want to give users a *taste* of it sooner, though!)
