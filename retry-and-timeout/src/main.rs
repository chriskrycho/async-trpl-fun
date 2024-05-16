use std::{
    future::Future,
    num::NonZeroU8,
    pin::{pin, Pin},
    time::Duration,
};

use futures::future;
use rand::Rng;
use tokio::time;

#[tokio::main]
async fn main() {
    match retry(NonZeroU8::new(10).unwrap(), Duration::from_millis(10), run).await {
        Ok((result, retries)) => println!("Resolved to '{result}' after {retries} (re)tries."),
        Err(error) => match error {
            Error::GaveUp { total } => println!("Gave up after {}ms", total.as_millis()),
            Error::Failed(timeout) => println!("Timed out after {}", timeout.as_millis()),
        },
    }
}

fn run() -> Box<dyn Future<Output = Result<String, Duration>>> {
    let mut get_delay = get_random_delay_milliseconds(NonZeroU8::new(10).unwrap());
    let result = timeout(get_delay(), async move {
        time::sleep(get_delay()).await;
        String::from("Tada")
    });
    Box::new(result)
}

fn get_random_delay_milliseconds(max: NonZeroU8) -> impl FnMut() -> Duration {
    let mut rng = rand::thread_rng();
    move || {
        let ms = rng.gen_range(1..=max.get());
        Duration::from_millis(ms as u64)
    }
}

async fn timeout<F: Future>(duration: Duration, future: F) -> Result<F::Output, Duration> {
    let timer = pin!(time::sleep(duration));
    let fut = pin!(future);
    match future::select(timer, fut).await {
        future::Either::Left(_) => Err(duration),
        future::Either::Right((output, _)) => Ok(output),
    }
}

/// Exponential backoff up to some number of times
async fn retry<F, T, E>(
    max_times: NonZeroU8,
    delay_base: Duration,
    op: F,
) -> Result<(T, u8), Error<E>>
where
    F: Fn() -> Box<dyn Future<Output = Result<T, E>>>,
{
    let max_times = max_times.get();
    let mut delay = delay_base;
    let backoff_ms = 2u128;
    for count in NonZeroU8::MIN.get()..=max_times {
        match Pin::from(op()).await {
            Ok(value) => return Ok((value, count)),
            Err(reason) => {
                if count >= max_times {
                    return Err(Error::Failed(reason));
                }

                println!("Failed, trying again after {}ms", delay.as_millis());
                time::sleep(delay).await;

                let new_delay = (delay.as_millis() + backoff_ms.pow(count as u32)) as u64;
                delay = Duration::from_millis(new_delay);
            }
        }
    }

    Err(Error::GaveUp { total: delay })
}

enum Error<E> {
    GaveUp { total: Duration },
    Failed(E),
}
