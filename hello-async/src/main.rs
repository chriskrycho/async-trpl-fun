use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use futures::executor;

fn main() {
    let fut = hello_async();
    let pinned_fut = pin!(fut);
    let waker = get_waker();
    let mut ctx: Context = Context::from_waker(&waker);

    loop {
        match pinned_fut.poll(&mut ctx) {
            Poll::Ready(_) => {
                break;
            }
            Poll::Pending => {
                // continue!
            }
        }
    }
    executor::block_on(hello_async());
}

async fn hello_async() {
    println!("Hello, world!");
}

fn get_waker() -> Waker {
    todo!()
}
