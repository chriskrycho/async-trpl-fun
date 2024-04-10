use std::path::Path;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::{sync::mpsc, task};

pub async fn watch(dir: &Path) -> Result<(), Error> {
    let (tx, mut rx) = mpsc::channel(8);

    // Doing this here means we will not drop the watcher until this function
    // ends, and the `while let` below will continue until there is an error (or
    // something else shuts down the whole system here!).
    let mut watcher = RecommendedWatcher::new(
        move |result| {
            if let Err(e) = tx.try_send(result) {
                eprintln!("Could not handle error gracefully.\nError:{e}");
            }
        },
        Config::default(),
    )
    .map_err(Error::from)?;

    watcher
        .watch(dir, RecursiveMode::Recursive)
        .map_err(Error::from)?;

    while let Some(result) = rx.recv().await {
        match result {
            Ok(event) => {
                println!("Got event! {:?}", event.kind);
            }
            Err(reason) => return Err(Error::from(reason)),
        }
    }

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Notify {
        #[from]
        source: notify::Error,
    },

    #[error(transparent)]
    Tokio {
        #[from]
        source: task::JoinError,
    },
}
