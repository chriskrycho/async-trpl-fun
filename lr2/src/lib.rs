use std::path::PathBuf;

use notify::{recommended_watcher, RecursiveMode, Watcher};
use tokio::{
    sync::mpsc::{self, Sender},
    task,
};

pub async fn watch(dir: PathBuf, event_channel: Sender<Change>) -> Result<(), Error> {
    let (tx, mut rx) = mpsc::channel(256);

    // Doing this here means we will not drop the watcher until this function
    // ends, and the `while let` below will continue until there is an error (or
    // something else shuts down the whole system here!).
    let mut watcher = recommended_watcher(move |result| {
        if let Err(e) = tx.blocking_send(result) {
            eprintln!("Could not send event.\nError:{e}");
        }
    })
    .map_err(Error::from)?;

    watcher
        .watch(&dir, RecursiveMode::Recursive)
        .map_err(Error::from)?;

    while let Some(result) = rx.recv().await {
        match result {
            Ok(event) => {
                eprintln!("Got an event:\n\t{:?}\n\t{:?}", event.kind, event.source());
                let change = Change { paths: event.paths };
                if let Err(e) = event_channel.send(change).await {
                    eprintln!("Error sending out: {e:?}");
                }
            }
            Err(reason) => return Err(Error::from(reason)),
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct Change {
    pub paths: Vec<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Notify {
        #[from]
        source: notify::Error,
    },

    #[error("{0}")]
    NotifyPlural(NotifyReasons),

    #[error(transparent)]
    Tokio {
        #[from]
        source: task::JoinError,
    },
}

impl From<Vec<notify::Error>> for Error {
    fn from(value: Vec<notify::Error>) -> Self {
        Error::NotifyPlural(NotifyReasons { sources: value })
    }
}

#[derive(Debug)]
pub struct NotifyReasons {
    sources: Vec<notify::Error>,
}

impl std::fmt::Display for NotifyReasons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.sources
                .iter()
                .map(|source| format!("{source:?}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}
