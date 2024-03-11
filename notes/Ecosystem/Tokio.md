---
aliases:
  - Tokio
---
- async main with `#[tokio::main] async fn main() -> Result<()>`
- analogous in *very* broad strokes to `async function main() { ... }` and then `await main()` in a Node thing.
- tasks are a single allocation and require only 64 *bytes* of memory.
    - For comparison, when you call `clone()` on Linux, you usually end up with *at least* 4KB because it allocates a whole page.
- supplies `Async*` versions of the `std::io` objects. (`smol` has this too, in the form of its [async-io](https://github.com/smol-rs/async-io) library.)
- minimal but necessary discussion of contention in [Shared State](https://tokio.rs/tokio/tutorial/shared-state): note that the concerns are basically the same here as they are in multi-threaded code in general, and the tradeoffs with alternatives (message passing, e.g.) are also similar.

This may be a general observation about the behavior of async runtimes in Rust, but it *definitely* applies to Tokio specifically: there are a *lot* of ways to trigger runtime panics by not doing things in the “right” order. 😒 Example from the `enter` and `spawn` docs:

> #### `pub fn enter(&self) -> EnterGuard<'_>`
> 
> Enters the runtime context.
> 
> This allows you to construct types that must have an executor available on creation such as Sleep or TcpStream. It will also allow you to call methods such as tokio::spawn.
>
> ##### Example
> 
> ```rust
> use tokio::runtime::Runtime;
> 
> fn function_that_spawns(msg: String) {
>     // Had we not used `rt.enter` below, this would panic.
>     tokio::spawn(async move {
>         println!("{}", msg);
>     });
> }
> 
> fn main() {
>     let rt = Runtime::new().unwrap();
> 
>     let s = "Hello World!".to_string();
> 
>     // By entering the context, we tie `tokio::spawn` to this executor.
>     let _guard = rt.enter();
>     function_that_spawns(s);
> }
> ```

The big thing to notice here is: this is classic “spooky action at a distance”. The runtime is being configured in some global sense, and there is no way to know *at the call site* for `tokio::spawn` whether things are set up correctly or not. You have to wait till runtime to find out whether you did it right.

On the one hand, this makes calling `tokio::spawn` a bit “nicer” in that you can do it without passing around some kind of token type or handle for the runtime. On the other, it means there is no way to actually guarantee you have done it right. That is a bit of a surprise; it runs against how Rust programs are normally constructed. (Notably, [[Ecosystem/smol]] does *not* work this way.) And, to be fair, Tokio expects you to usually just use the `#[tokio::main]`  macro to handle setting this up once in your main function, such that you never have to worry about it. In the cases you *do* want to care about it you can use `Runtime::spawn` (from `tokio::runtime`) instead of `tokio::spawn`, and get the same rough behavior you would from e.g. smol. And you could, in principle, lint to require *either* `#[tokio::main]` *or* using `Runtime::spawn` or something like that.

You could argue this is basically “progressive disclosure of complexity” to make the base case trivial. I still don’t love it. It still feels like a bit of a footgun!

## Questions

- Why did they choose the `Async(Read|Write)` and `Async(Read|Write)Ext` pattern, instead of doing the same thing that `Iterator` does and implementing it directly on the trait?
- 