use std::{
    future::Future,
    pin::Pin,
    sync::{mpsc, Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};

use futures::task::{self, ArcWake};

pub fn run<F>(fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let mini_tokio = MiniTokio::new();

    mini_tokio.spawn(fut);
    mini_tokio.run();
}

struct MiniTokio {
    scheduled: mpsc::Receiver<Arc<Task>>,
    sender: mpsc::Sender<Arc<Task>>,
}

impl MiniTokio {
    fn run(&self) {
        // Use `recv_timeout` so the rest of my little test harness in `main`
        // works; the tutorial does not need that. Otherwise, this just keeps
        // going until interrupted, as a “real” runtime would.
        while let Ok(task) = self.scheduled.recv_timeout(Duration::from_secs(1)) {
            task.poll();
        }
    }

    fn new() -> MiniTokio {
        let (sender, scheduled) = mpsc::channel();

        MiniTokio { sender, scheduled }
    }

    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Task::spawn(future, &self.sender);
    }
}

/// A future and the result of the latest call to its `poll` method.
struct TaskFuture {
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    poll: Poll<()>,
}

impl TaskFuture {
    fn new(future: impl Future<Output = ()> + Send + 'static) -> TaskFuture {
        TaskFuture {
            future: Box::pin(future),
            poll: Poll::Pending,
        }
    }

    fn poll(&mut self, cx: &mut Context<'_>) {
        // Spurious wake-ups are allowed, even after a future has returned
        // `Ready`. However, polling a future which has already returned `Ready`
        // is *not* allowed. For this reason we need to check that the future is
        // still pending before we call it. Failure to do so can lead to a
        // panic. [This comment copied straight from the tutorial.]
        if self.poll.is_pending() {
            self.poll = self.future.as_mut().poll(cx);
        }
    }
}

struct Task {
    // The `Task` needs to be `Sync`, and `Mutex` is a shortcut way to do it; it
    // is possible to implement in other ways correctly *without* the overhead
    // of a mutex, since only one thread accesses the `task_future` at a time.
    task_future: Mutex<TaskFuture>,
    executor: mpsc::Sender<Arc<Task>>,
}

impl Task {
    fn poll(self: Arc<Self>) {
        // `futures::task::waker` uses `ArcWake`, so this will call the impl
        // for `Task` below.
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);

        // In *this* implementation, this only happens on one thread, so just
        // `.unwrap()` (this could panic if that were *not* true).
        let mut task_future = self.task_future.try_lock().unwrap();

        // This is not the `Future::poll` but the `TaskFuture::poll` implemented
        // directly above. *That* is what actually calls `Future::poll()`.
        task_future.poll(&mut cx);
    }

    // Notice that the task uses the mspc channel to *schedule itself* here. The
    // channel will dump it into `MiniTokio::scheduled` (the receiver). The
    // chain to call this is:
    //
    // `Task::poll()`
    //      -> `task::waker()
    //      -> `impl ArcWake for Task`
    //      -> here
    //
    // After this, `Task::poll()` keeps running to do other things. So this
    // `Task` instance gets put on the schedule and then *separately* calls its
    // own `task_future`’s `poll()` function, which in turn ultimately calls
    // `Future::poll()` if it is still pending.
    fn schedule(self: &Arc<Self>) {
        let _ = self.executor.send(self.clone());
    }

    fn spawn<F>(future: F, sender: &mpsc::Sender<Arc<Task>>)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Arc::new(Task {
            task_future: Mutex::new(TaskFuture::new(future)),
            executor: sender.clone(),
        });

        let _ = sender.send(task);
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.schedule();
    }
}
