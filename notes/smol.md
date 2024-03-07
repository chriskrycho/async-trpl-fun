An alternative, smaller 🙄 async runtime which aims to feel similar to Tokio but is (or at least claims to be?) much lighter weight. Also makes some better (in my view, anyway) API choices.

For example, `smol::spawn` will lazily instantiate a global singleton executor if you call it directly, but it recommends using `Executor::spawn` or `LocalExecutor::spawn`, which have the API design I would actually expect:

```rust
use async_executor::Executor;

let ex = Executor::new();

let task = ex.spawn(async {
    println!("Hello world");
});
```

One potential “downside” here is that you now have to manage ownership explicitly. That said, you also get *control* with that. (In the Tokio case, you don’t actually *need* to do it most of the time because of `#[tokio::main]`, *and* you can use `Runtime::spawn` instead of `tokio::spawn` to be safe.)