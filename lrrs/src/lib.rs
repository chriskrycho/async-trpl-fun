use std::{net::SocketAddr, path::Path, pin::pin};

use axum::Router;
use futures::future::join;
use tokio::{net::TcpListener, runtime::Runtime, signal::ctrl_c, task::JoinError};
use tokio_util::sync::CancellationToken;
use tower_http::services::ServeDir;
use watchexec::{error::CriticalError, Watchexec};
use watchexec_signals::Signal;

pub fn serve(path: impl AsRef<Path>) -> Result<(), Error> {
    let rt = Runtime::new().map_err(|e| Error::Io { source: e })?;

    let token = CancellationToken::new();

    rt.spawn({
        // What we actually want is this *from the watch*, right?
        let token = token.clone();

        async move {
            if let Ok(()) = ctrl_c().await {
                println!("Interrupt!");
                token.cancel();
            }
        }
    });

    rt.block_on(async {
        let watch = watcher_in(path.as_ref());
        let serve = serve_in(path.as_ref());

        // `.await`s both internally.
        let watches = join(watch, serve);

        tokio::select! {
            res = watches => match res {
                (Ok(_), Ok(_)) => todo!(),
                (Ok(_), Err(_)) => todo!(),
                (Err(_), Ok(_)) => todo!(),
                (Err(_), Err(_)) => todo!(),
            },
            _ = ctrl_c() => {
                println!("canceling!");
                token.cancel();
            }
        }
    });

    Ok(())
}

async fn serve_in(path: &Path) -> Result<(), Error> {
    // This could be extracted into its own function.
    let serve_dir = ServeDir::new(path).append_index_html_on_directories(true);
    let router = Router::new().route_service("/*asset", serve_dir);

    let addr = SocketAddr::from(([127, 0, 0, 1], 9876));
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| Error::BadAddress {
            value: addr,
            source: e,
        })?;

    eprintln!("→ Serving at: http://{addr}");

    axum::serve(listener, router)
        .await
        .map_err(|e| Error::ServeStart { source: e })
}

async fn watcher_in(path: &Path) -> Result<(), Error> {
    let watcher = Watchexec::new(|action_handler| {
        // This needs `.iter()` because `events` is an `Arc<[Event]>`, not just
        // `[Event]`, so `.iter()` delegates to the inner bit.
        for event in action_handler.events.iter() {
            eprintln!("Event: {event:#?}");
        }

        action_handler
    })
    .map_err(Error::from)?;

    eprintln!("watching for changes in {}", path.display());
    watcher.config.pathset([path]);

    watcher
        .main()
        .await
        .map_err(|e| Error::WatchEnd { source: e })
        .and_then(|result| result.map_err(|e| Error::Watch { source: e }))
}

#[derive(Debug, thiserror::Error)]
#[error("Error serving site")]
pub enum Error {
    #[error("I/O error")]
    Io { source: std::io::Error },

    #[error("Error starting file watcher")]
    Watch {
        #[from]
        source: CriticalError,
    },

    #[error("Could not open socket on address: {value}")]
    BadAddress {
        value: SocketAddr,
        source: std::io::Error,
    },

    #[error("Could not start the site server")]
    ServeStart { source: std::io::Error },

    #[error("Runtime error")]
    Tokio {
        #[from]
        source: JoinError,
    },

    #[error("Watch error")]
    WatchEnd { source: JoinError },
}
