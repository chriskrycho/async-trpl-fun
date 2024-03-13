use std::{
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;
use tokio::sync::oneshot;

#[tokio::main]
async fn main() {
    // Tokio’s `Receiver` type implements `Future`, so `rx1` and `rx2` are
    // `.await`-able. This means that they can be `select!`ed on directly, with
    // no extra ceremony: see below!
    let (tx1, rx1) = oneshot::channel();
    let (tx2, rx2) = oneshot::channel();

    tokio::spawn(async {
        let _ = tx1.send("Hello");
    });

    tokio::spawn(async {
        let _ = tx2.send("World");
    });

    tokio::select! {
        val = rx1 => {
            println!("rx1 completed first with '{val:?}'");
        }
        val = rx2 => {
            println!("rx2 completed first with '{val:?}'");
        }
    }
}

struct MySelect<A, B>
where
    A: Debug,
    B: Debug,
{
    rx1: oneshot::Receiver<A>,
    rx2: oneshot::Receiver<B>,
}

impl<A, B> Future for MySelect<A, B>
where
    A: Debug,
    B: Debug,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        if let Poll::Ready(val) = Pin::new(&mut self.rx1).poll(cx) {
            println!("rx1 completed with {val:?}");
            return Poll::Ready(());
        }

        if let Poll::Ready(val) = Pin::new(&mut self.rx2).poll(cx) {
            println!("rx2 completed with {val:?}");
            return Poll::Ready(());
        }

        Poll::Pending
    }
}
