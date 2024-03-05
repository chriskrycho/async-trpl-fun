use std::{path::Path, time::Duration};

// use fastwebsockets::WebSocket;
use tokio::task::JoinError;
use watchexec::{error::CriticalError, sources::fs::WatchedPath, Watchexec};
use watchexec_signals::Signal;

pub async fn live_reload(sources: &[impl AsRef<Path>]) -> Result<(), Error> {
    let watch = Watchexec::new(|mut handler| {
        // This needs `.iter()` because `events` is an `Arc<[Event]>`, not just
        // `[Event]`, so `.iter()` delegates to the inner bit.
        for event in handler.events.iter() {
            println!("Event: {event:#?}");
        }

        if handler.signals().any(|sig| sig == Signal::Interrupt) {
            handler.quit_gracefully(Signal::Interrupt, Duration::from_secs(1));
        }

        handler
    })
    .map_err(Error::Watch)?;

    watch.config.pathset(
        sources
            .iter()
            .map(|source| WatchedPath::from(source.as_ref())),
    );

    watch
        .main()
        .await
        .map_err(Error::Tokio)
        .and_then(|inner| inner.map_err(Error::Watch))
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    #[error("watch error: {0}")]
    Watch(CriticalError),

    #[error(transparent)]
    Tokio(JoinError),
}
