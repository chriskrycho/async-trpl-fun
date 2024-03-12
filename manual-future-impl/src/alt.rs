use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

pub enum MainFuture {
    State0,
    State1(Delay),
    Terminated,
}

impl Future for MainFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use MainFuture::*;

        loop {
            match *self {
                State0 => {
                    let future = Delay::until(Instant::now() + Duration::from_millis(10));
                    *self = State1(future);
                }
                State1(ref mut delay_future) => match Pin::new(delay_future).poll(cx) {
                    Poll::Ready(out) => {
                        assert_eq!(out, "done");
                        *self = Terminated;
                        return Poll::Ready(());
                    }
                    Poll::Pending => return Poll::Pending,
                },
                Terminated => {
                    panic!("future polled after completion!");
                }
            }
        }
    }
}

pub fn manual() {
    futures::executor::block_on(MainFuture::State0);
}

pub struct Delay {
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
            println!("AHOY, COSMOS!");
            Poll::Ready("done")
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
