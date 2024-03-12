mod alt;
mod mini_tokio;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() {
    mini_tokio::run(async {
        let out = Delay::until(Instant::now() + Duration::from_millis(10)).await;
        println!("Hello, from mini_tokio");
        assert_eq!(out, ());
    });

    alt::manual();

    let future = Delay::until(Instant::now() + Duration::from_millis(10));
    let out = future.await;
    println!("Hello, from regular tokio");
    assert_eq!(out, ());
}

struct Delay {
    when: Instant,
    waker: Option<Arc<Mutex<Waker>>>,
}

impl Delay {
    fn until(time: Instant) -> Delay {
        Delay {
            when: time,
            waker: None,
        }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // If everything is already done, there is no need for extra handling.
        if Instant::now() >= self.when {
            return Poll::Ready(());
        }

        // Otherwise, schedule the work. If this is the first time the `Future`
        // is called, we should spawn the timer thread. If it is *not* the first
        // time, the thread should already exist; make sure the stored `Waker`
        // matches the current task’s waker.
        if let Some(waker) = &self.waker {
            let mut waker = waker.lock().unwrap();

            // This is the check for whether the wakers “match”. The `Delay` can
            // move to different tasks between calls to `poll`, for example if
            // the runtime needs to for scheduling reasons *or* if it gets moved
            // via `async move { ... }` on the user side. If it has been moved,
            // the waker from the context will be different, so we switch to use
            // that one instead of the original (which gets abandoned).
            if !waker.will_wake(cx.waker()) {
                *waker = cx.waker().clone();
            }
        } else {
            // To avoid blocking the thread (because otherwise the executor will
            // just keep calling `poll`) spawn a *different* thread and wait
            // over there until the time has passed. Note that in a real
            // implementation, this would go on a thread pool or something like
            // that to avoid spawning an arbitrary number of threads for work!
            let when = self.when;
            let waker = Arc::new(Mutex::new(cx.waker().clone()));
            self.waker = Some(waker.clone()); // push in a reference only…

            // …because we also need to hand the waker to the thread.
            // Needs to be `move` because it is going to take ownership of the
            // waker, i.e. move it to the spawned thread.
            thread::spawn(move || {
                let now = Instant::now();

                // Not time yet? Sleep till it is.
                if now < when {
                    thread::sleep(when - now);
                }

                // Otherwise, notify the caller via the waker.
                let waker = waker.lock().unwrap();
                waker.wake_by_ref();
            });
        }

        // Here, the waker is stored correctly and the timer thread is started,
        // but the duration has not elapsed, since that was the first thing the
        // function checks. Thus the result *must* be `Pending`.
        //
        // The trait contract requires that the waker for the current context be
        // signaled once it is ready when you return `Pending` like this, so
        // is a promise that the waker from the most recent `Context` will be
        // called once the time elapses, to avoid hanging forever.
        Poll::Pending
    }
}
