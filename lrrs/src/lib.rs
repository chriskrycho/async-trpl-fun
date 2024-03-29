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

    let (msg_tx, mut msg_rx) = broadcast::channel(100);
    let (change_tx, _) = broadcast::channel(10);

    let shared = Arc::new(SharedState {
        msg: msg_tx,
        change: change_tx,
    });

    let serve = rt.spawn(server_in(path.to_owned(), shared.clone()));
    let watch = rt.spawn(watcher_in(path.to_owned(), shared.clone()));

    let close_signal = rt.spawn(async move { ctrl_c().await.map_err(|v| Error::Io { source: v }) });

    let coordinate = rt.spawn(async move {
        // The goal here is to make progress *unless*
        loop {
            eprintln!("starting up the loop");

            match msg_rx.recv().await {
                Ok(Msg::Close { reason }) => {
                    println!("closed ws: {}", reason.unwrap_or("unknown".into()));
                }
                Ok(Msg::Error { reason }) => {
                    eprintln!("uh oh: {reason}");
                    return Err(Error::Handled(reason));
                }
                Err(e) => {
                    return Err(Error::Receive { source: e });
                }
            }
        }
    });

    rt.block_on(select_ok([serve, watch, coordinate, close_signal]))
        .map_err(|join_err| Error::TopLevel { source: join_err })
        .and_then(|(result, _rest)| result)
}

async fn server_in(path: PathBuf, state: Arc<SharedState>) -> Result<(), Error> {
    // This could be extracted into its own function.
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

#[derive(Debug)]
struct SharedState {
    change: broadcast::Sender<FileChanged>,
    msg: broadcast::Sender<Msg>,
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<Arc<SharedState>>) -> Response {
    eprintln!("upgrading the websocket");
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Arc<SharedState>) {
    let (mut ws_sender, mut receiver) = stream.split();
    let mut change = state.change.subscribe();

    loop {
        // For now, ignore error case
        if (change.recv().await).is_ok() {
            eprint!("sending ws message…");
            ws_sender
                .send(WsMessage::Text(String::from("reload")))
                .await
                .unwrap(); // TODO: error handling!

            eprintln!(" done.");
        }

        if let Some(message) = receiver.next().await {
            if let Some(msg) = ws_message(message) {
                eprint!("sending internal message…");
                state.msg.send(msg).unwrap(); // TODO: error handling!
                eprintln!(" done.");
            }
        } else {
            break;
        }
    }
}

fn ws_message(message: Result<WsMessage, axum::Error>) -> Option<Msg> {
    eprintln!("got {message:?} from websocket");

    use WsMessage::*;
    match message {
        Ok(message) => match message {
            // We don't care about *receiving* messages from the websocket, only
            // sending them *to* it.
            Text(_) | Binary(_) | Ping(_) | Pong(_) => None,

            // We *do* care if the socket closes. (Maybe? Only for logging, at
            // the moment.)
            Close(maybe_frame) => {
                let message = Msg::Close {
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

                Some(message)
            }
        },

        Err(reason) => {
            let message = Msg::Error {
                reason: reason.to_string(),
            };

            Some(message)
        }
    }
}

// Could later wrap type of change.
#[derive(Debug, Clone)]
struct FileChanged;

#[derive(Debug, Clone)]
enum Msg {
    Close { reason: Option<String> },
    Error { reason: String },
}

async fn watcher_in(path: PathBuf, state: Arc<SharedState>) -> Result<(), Error> {
    let close_state = state.clone();
    let watcher = Watchexec::new(move |mut handler| {
        if handler.signals().any(|signal| signal == Signal::Interrupt) {
            if let Err(reason) = close_state.msg.send(Msg::Close {
                reason: Some(String::from("Interrupt!")),
            }) {
                eprintln!("Could not close channel gracefully; hard closing it.\n{reason}");
                handler.quit();
            }
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
            let has_listeners = future_state.change.receiver_count() > 0;
            if should_reload && has_listeners {
                future_state
                    .change
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

    #[error("Runtime error: {source}")]
    Tokio {
        #[from]
        source: JoinError,
    },

    #[error("Watch error: {source}")]
    WatchEnd { source: JoinError },

    #[error("Receiving internal message: {source}")]
    Receive { source: RecvError },

    #[error("Top-level error: {source}")]
    TopLevel { source: JoinError },

    #[error("{0}")]
    Handled(String),
}
