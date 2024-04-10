use std::{path::PathBuf, time::Duration};

use notify_debouncer_full::{
    new_debouncer,
    notify::{RecursiveMode, Watcher},
    DebouncedEvent,
};
use tokio::{
    sync::mpsc::{self, Sender},
    task,
};

pub async fn watch(
    dir: PathBuf,
    event_channel: Sender<Change>,
    timeout: Duration,
) -> Result<(), Error> {
    let (tx, mut rx) = mpsc::channel(256);

    // Doing this here means we will not drop the watcher until this function
    // ends, and the `while let` below will continue until there is an error (or
    // something else shuts down the whole system here!).
    let mut debounced = new_debouncer(timeout, None, move |result| {
        if let Err(e) = tx.blocking_send(result) {
            eprintln!("Could not send event.\nError:{e}");
        }
    })
    .map_err(Error::from)?;

    debounced
        .watcher()
        .watch(&dir, RecursiveMode::Recursive)
        .map_err(Error::from)?;

    while let Some(result) = rx.recv().await {
        match result {
            Ok(debounced_events) => {
                for DebouncedEvent { event, .. } in debounced_events {
                    let change = Change { paths: event.paths };
                    if let Err(e) = event_channel.send(change).await {
                        eprintln!("Error sending out: {e:?}");
                    }
                }
            }
            Err(reasons) => return Err(Error::from(reasons)),
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
