mod alt;
mod mini_tokio;

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
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
        if Instant::now() >= self.when {
            println!("Hello, world!");
            Poll::Ready("done")
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
