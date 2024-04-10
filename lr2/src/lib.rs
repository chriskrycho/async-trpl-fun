use std::path::Path;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::{sync::mpsc, task::JoinSet};

pub async fn watch(dir: &Path) -> Result<(), Error> {
    let (events_tx, mut events_rx) = mpsc::channel(1024);
    let (errors_tx, mut errors_rx) = mpsc::channel(1024);

    // This needs to be here simply so that the watcher continues to exist until
    // the rest of the operations stop yielding values.
    let _watcher = watch_notify(dir, events_tx.clone(), errors_tx.clone())?;

    let mut set = JoinSet::new();
    set.spawn(async move {
        while let Some(evt) = events_rx.recv().await {
            println!("Got event! {:?}", evt.kind);
        }
    });

    set.spawn(async move {
        while let Some(err) = errors_rx.recv().await {
            eprintln!("Got error! {err}");
        }
    });

    while let Some(result) = set.join_next().await {
        match result {
            Ok(_) => println!("Succeeded."),
            Err(reason) => eprintln!("Failed: {reason}"),
        }
    }

    Ok(())
}

fn watch_notify(
    dir: &Path,
    events: mpsc::Sender<Event>,
    errors: mpsc::Sender<Error>,
) -> Result<RecommendedWatcher, Error> {
    let mut watcher = RecommendedWatcher::new(
        move |watch_result| match watch_result {
            Ok(event) => {
                if let Err(e) = events.try_send(event) {
                    eprintln!("Could not handle error gracefully.\nError:{e}");
                }
            }
            Err(source) => {
                if let Err(e) = errors.try_send(Error::Notify { source }) {
                    eprintln!("Could not handle error gracefully.\nError:{e}");
                }
            }
        },
        Config::default(),
    )
    .map_err(Error::from)?;

    watcher
        .watch(dir, RecursiveMode::Recursive)
        .map_err(Error::from)?;

    Ok(watcher)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Notify {
        #[from]
        source: notify::Error,
    },
}
