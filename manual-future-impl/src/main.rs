mod alt;
mod mini_tokio;

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    thread,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() {
    mini_tokio::run(async {
        let out = Delay::until(Instant::now() + Duration::from_millis(10)).await;
        assert_eq!(out, "done");
    });

    alt::manual();

    let future = Delay::until(Instant::now() + Duration::from_millis(10));

    let out = future.await;
    assert_eq!(out, "done");
}

struct Delay {
    when: Instant,
}

impl Delay {
    fn until(time: Instant) -> Delay {
        Delay { when: time }
    }
}

impl Future for Delay {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // In the first case, everything is already done, so there is no need
        // for extra handling. In the second case, though, to avoid blocking the
        // thread (because otherwise the executor will just keep calling `poll`)
        // spawn a *different* thread and wait over there until the time has
        // passed. Note that in a real implementation, this would go on a thread
        // pool or something like that to avoid spawning an arbitrary number of
        // threads for work!
        if Instant::now() >= self.when {
            println!("Hello, world!");
            Poll::Ready("done")
        } else {
            let waker = cx.waker().clone();
            let when = self.when;

            // Needs to be `move` because it is going to take ownership of the
            // waker, i.e. move it to the spawned thread.
            thread::spawn(move || {
                let now = Instant::now();

                if now < when {
                    thread::sleep(when - now);
                }

                waker.wake();
            });

            Poll::Pending
        }
    }
}
