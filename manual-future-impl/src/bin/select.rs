use std::{
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;
use tokio::sync::oneshot;

#[tokio::main]
async fn main() {
    let (tx1, rx1) = oneshot::channel::<String>();
    let (tx2, rx2) = oneshot::channel::<String>();

    tokio::spawn(async {
        let _ = tx1.send("Hello".into());
    });

    tokio::spawn(async {
        let _ = tx2.send("World".into());
    });

    MySelect { rx1, rx2 }.await
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
