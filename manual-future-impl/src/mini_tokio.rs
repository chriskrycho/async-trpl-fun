use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Wake},
    time::{Duration, Instant},
};

use futures::task;

pub fn run<F>(fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let mut mini_tokio = MiniTokio::new();

    mini_tokio.spawn(fut);
    mini_tokio.run();
}

struct MiniTokio {
    tasks: VecDeque<Task>,
}

type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

impl MiniTokio {
    fn new() -> MiniTokio {
        MiniTokio {
            tasks: VecDeque::new(),
        }
    }

    fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tasks.push_back(Box::pin(future))
    }

    fn run(&mut self) {
        let waker = task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        while let Some(mut task) = self.tasks.pop_front() {
            if task.as_mut().poll(&mut cx).is_pending() {
                self.tasks.push_back(task);
            }
        }
    }
}
