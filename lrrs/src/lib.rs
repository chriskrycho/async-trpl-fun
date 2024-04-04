use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{future::select_ok, SinkExt, StreamExt};
use tokio::{
    net::TcpListener,
    runtime::Runtime,
    signal::ctrl_c,
    sync::broadcast::{self, error::RecvError},
    task::JoinError,
};
use tower_http::services::ServeDir;
use watchexec::{error::CriticalError, Watchexec};
use watchexec_signals::Signal;

pub fn serve(path: PathBuf) -> Result<(), Error> {
    let rt = Runtime::new().map_err(|e| Error::Io { source: e })?;

    let (tx, _) = broadcast::channel(10);

    let shared = Arc::new(tx);

    let serve = rt.spawn(server_in(path.to_owned(), shared.clone()));
    let watch = rt.spawn(watcher_in(path.to_owned(), shared.clone()));
    let close = rt.spawn(async move { ctrl_c().await.map_err(|v| Error::Io { source: v }) });

    rt.block_on(select_ok([serve, watch, close]))
        .map_err(|join_err| Error::TopLevel { source: join_err })
        .and_then(|(result, _rest)| result)
}

async fn server_in(path: PathBuf, state: Shared) -> Result<(), Error> {
    let serve_dir = ServeDir::new(path).append_index_html_on_directories(true);

    let router = Router::new()
        .nest_service("/", serve_dir)
        .route("/lr", get(ws_upgrade))
        .with_state(state);

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

type Shared = Arc<broadcast::Sender<FileChanged>>;

async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<Shared>) -> Response {
    eprintln!("upgrading the websocket");
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Shared) {
    let (mut ws_sender, mut receiver) = stream.split();
    let mut change = state.subscribe();

    let mut listening = true;

    loop {
        match change.recv().await {
            Ok(FileChanged) => {
                if listening {
                    eprint!("sending WebSocket reload message…");
                    ws_sender
                        .send(WsMessage::Text(String::from("reload")))
                        .await
                        .unwrap(); // TODO: error handling!

                    eprintln!(" done.");
                }
            }
            Err(recv_error) => match recv_error {
                RecvError::Closed => todo!(),
                RecvError::Lagged(dropped_count) => {
                    eprintln!("Dropped {dropped_count} messages from internal queue");
                }
            },
        }

        if let Some(websocket_message) = receiver.next().await {
            match handle(websocket_message) {
                Ok(Some(WebSocketClosed { reason })) => {
                    eprintln!(
                        "WebSocket instance closed: {}",
                        reason.unwrap_or(String::from("(reason unknown)"))
                    );
                    listening = false;
                }
                Ok(None) => (/* no-op */),
                Err(reason) => {
                    eprintln!("WebSocket error: {reason}");
                    listening = false;
                }
            }
        } else {
            break;
        }
    }
}

fn handle(message: Result<WsMessage, axum::Error>) -> Result<Option<WebSocketClosed>, Error> {
    eprintln!("got {message:?} from websocket");

    use WsMessage::*;
    match message {
        Ok(message) => match message {
            // We don't care about *receiving* messages from the websocket, only
            // sending them *to* it.
            Text(_) | Binary(_) | Ping(_) | Pong(_) => Ok(None),

            // We *do* care if the socket closes. (Maybe? Only for logging, at
            // the moment.)
            Close(maybe_frame) => {
                let message = WebSocketClosed {
                    reason: maybe_frame.map(|frame| {
                        let desc = if !frame.reason.is_empty() {
                            format!("Reason: {};", frame.reason)
                        } else {
                            String::from("")
                        };

                        let code = format!("Code: {}", frame.code);
                        desc + &code
                    }),
                };

                Ok(Some(message))
            }
        },

        Err(reason) => Err(Error::Serve { source: reason }),
    }
}

// Could later wrap type of change.
#[derive(Debug, Clone)]
struct FileChanged;

#[derive(Debug, Clone)]
struct WebSocketClosed {
    reason: Option<String>,
}

async fn watcher_in(path: PathBuf, state: Shared) -> Result<(), Error> {
    let watcher = Watchexec::new(move |mut handler| {
        if handler.signals().any(|signal| signal == Signal::Interrupt) {
            eprintln!("Attempting to quit watch handler");
            handler.quit();
        }

        handler
    })
    .map_err(Error::from)?;

    watcher.config.pathset([&path]);
    watcher.config.on_action_async(move |handler| {
        // Although we have moved the `shared` pointer into the closure by using
        // a `move` closure, that reference will be dropped at the end of the
        // closure, since it is not returned. The future outlives the closure,
        // though, so needs its own reference.
        let future_state = state.clone();

        // That reference needs to be owned by the future, so use `async move`
        // to move ownership.
        let future = async move {
            // Only send a notice when there is a relevant change *and* when
            // something is listening for it.
            let should_reload = handler.events.iter().any(|event| event.paths().count() > 0);
            let has_listeners = future_state.receiver_count() > 0;
            if should_reload && has_listeners {
                future_state
                    .send(FileChanged)
                    .expect("The `Sender` must always have at least one `Receiver`");
            }

            handler
        };

        Box::new(future)
    });

    eprintln!("watching for changes in {}", path.display());

    watcher
        .main()
        .await
        .map_err(|e| Error::WatchEnd { source: e })
        .and_then(|result| result.map_err(|e| Error::Watch { source: e }))
}

#[derive(Debug, thiserror::Error)]
#[error("Error serving site")]
pub enum Error {
    #[error("I/O error: {source}")]
    Io { source: std::io::Error },

    #[error("Error starting file watcher: {source}")]
    Watch {
        #[from]
        source: CriticalError,
    },

    #[error("Could not open socket on address {value}: {source}")]
    BadAddress {
        value: SocketAddr,
        source: std::io::Error,
    },

    #[error("Could not start the site server: {source}")]
    ServeStart { source: std::io::Error },

    #[error("{source}")]
    Serve { source: axum::Error },

    #[error("Watch error: {source}")]
    WatchEnd { source: JoinError },

    #[error("Receiving internal message: {source}")]
    Receive { source: RecvError },

    #[error("Top-level error: {source}")]
    TopLevel { source: JoinError },

    #[error("{0}")]
    Handled(String),
}
