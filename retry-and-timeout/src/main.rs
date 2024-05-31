use std::{future::Future, num::NonZeroU8, pin::pin, time::Duration};

use futures::future::{self, TryFutureExt};
use rand::Rng;
use tokio::time;

#[tokio::main]
async fn main() {
    let max_times = NonZeroU8::new(10).unwrap();
    let base_delay = Duration::from_millis(10);
    match retry(max_times, base_delay, run).await {
        Ok((result, retries)) => println!("Resolved to '{result}' after {retries} (re)tries."),
        Err(GaveUp { total, source }) => println!(
            "Gave up after {}ms{}",
            total.as_millis(),
            source
                .map(|source| format!(": {source}"))
                .unwrap_or_default()
        ),
    }
}

fn run() -> impl Future<Output = Result<String, String>> {
    let mut get_delay = get_random_delay_milliseconds(NonZeroU8::new(10).unwrap());
    timeout(get_delay(), async move {
        time::sleep(get_delay()).await;
        String::from("Tada")
    })
    .map_err(|e| format!("Timed out after {}ms", e.as_millis()))
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
async fn retry<F, T, E, Fut>(
    max_times: NonZeroU8,
    delay_base: Duration,
    mut op: F,
) -> Result<(T, u8), GaveUp<E>>
where
    F: FnMut() -> Fut,
    E: std::fmt::Display,
    Fut: Future<Output = Result<T, E>>,
{
    let max_times = max_times.get();
    let mut delay = delay_base;
    let backoff_ms = 2u128;
    for count in NonZeroU8::MIN.get()..=max_times {
        match op().await {
            Ok(value) => return Ok((value, count)),
            Err(reason) => {
                if count >= max_times {
                    return Err(GaveUp {
                        total: delay,
                        source: Some(reason),
                    });
                }

                println!("Failed, trying again after {}ms", delay.as_millis());
                time::sleep(delay).await;

                let new_delay = (delay.as_millis() + backoff_ms.pow(count as u32)) as u64;
                delay = Duration::from_millis(new_delay);
            }
        }
    }

    Err(GaveUp {
        total: delay,
        source: None,
    })
}

struct GaveUp<E: std::fmt::Display> {
    total: Duration,
    source: Option<E>,
}
