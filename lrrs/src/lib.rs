use std::{net::SocketAddr, path::Path, sync::Arc};

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use tokio::{
    net::TcpListener,
    runtime::Runtime,
    signal::ctrl_c,
    sync::{broadcast, mpsc},
    task::JoinError,
};
use tower_http::services::ServeDir;
use watchexec::{error::CriticalError, Watchexec};
use watchexec_signals::Signal;

pub fn serve(path: impl AsRef<Path>) -> Result<(), Error> {
    let rt = Runtime::new().map_err(|e| Error::Io { source: e })?;

    let (tx, mut rx) = broadcast::channel(100);
    let (change_tx, _) = broadcast::channel(10);
    let (close_tx, mut close_rx) = mpsc::unbounded_channel();
    let shared = Arc::new(SharedState {
        tx,
        close: close_tx,
        change: change_tx,
    });

    let serve = serve_in(path.as_ref(), shared.clone());
    let watch = watcher_in(path.as_ref(), shared.clone());

    rt.block_on(async {
        eprintln!("starting up!");
        tokio::select! {
            // Handle the watch ending…
            res = watch => match res {
                Ok(()) => { eprintln!("ended watch with ok")},
                Err(error) => { eprintln!("ended watch with error:\n{error}")},
            },

            // …or the server ending.
            res = serve => match res {
                Ok(()) => { eprintln!("ended serve with ok")},
                Err(error) => { eprintln!("ended serve with error:\n{error}")},
            },

            // Allow any part of the program to close via signal…
            Some(()) = close_rx.recv() => {
                println!("canceling via broadcast channel (likely watchexec signal");
            },

            // …including Tokio’s top-level handling.
            _ = ctrl_c() => {
                println!("canceling via tokio::signal::ctrl_c !");
            },

            // And now for the actual good part: handling the message loop!
            _ = async {
                loop {
                    match rx.recv().await {
                        Ok(Msg::Receive { content }) => println!("{content}"),
                        Ok(Msg::Close { reason }) => {
                            println!("close: {}", reason.unwrap_or("unknown".into()));
                        },
                        Ok(Msg::Error { reason }) => {
                            eprintln!("uh oh: {reason}");
                            break
                        }
                        Err(e) => {
                            eprintln!("bad times: {e}");
                            break
                        },
                    }
                }

                Ok::<(), Error>(())
            } => {}
        }
    });

    Ok(())
}

async fn serve_in(path: &Path, state: Arc<SharedState>) -> Result<(), Error> {
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
    tx: broadcast::Sender<Msg>,
    close: mpsc::UnboundedSender<()>,
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<Arc<SharedState>>) -> Response {
    eprintln!("upgrading the websocket");
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Arc<SharedState>) {
    let (mut sender, mut receiver) = stream.split();
    let mut change = state.change.subscribe();

    loop {
        let next = tokio::select! {
            message = receiver.next() => match message {
                Some(message) => ws_message(message),
                None => break,
            },

            m = change.recv() => match m {
                Ok(_file_changed) => Next::Ws(WsMessage::Text(String::from("reload"))),
                Err(_recv_err) => Next::None, // TODO: error handling!
            },
        };

        match next {
            Next::Internal(internal) => {
                eprint!("sending internal message…");
                state.tx.send(internal).unwrap(); // TODO: error handling!
                eprintln!(" done.");
            }
            Next::Ws(reply) => {
                eprint!("sending ws message…");
                sender.send(reply).await.unwrap(); // TODO: error handling!
                eprintln!(" done.");
            }
            Next::None => { /* Nothing to do! */ }
        }
    }
}

enum Next {
    None,
    Internal(Msg),
    Ws(WsMessage),
}

fn ws_message(message: Result<WsMessage, axum::Error>) -> Next {
    eprintln!("got {message:?} from websocket");

    match message {
        Ok(WsMessage::Text(content)) => {
            println!("Got a message!: '{content}'");
            Next::Internal(Msg::Receive { content })
        }

        Ok(WsMessage::Binary(_)) => Next::Ws(WsMessage::Text(String::from(
            "Binary data is not supported",
        ))),

        Ok(WsMessage::Ping(_) | WsMessage::Pong(_)) => Next::None,

        Ok(WsMessage::Close(maybe_frame)) => {
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

            Next::Internal(message)
        }

        Err(reason) => {
            let message = Msg::Error {
                reason: reason.to_string(),
            };

            Next::Internal(message)
        }
    }
}

// Could later wrap type of change.
#[derive(Debug, Clone)]
struct FileChanged;

#[derive(Debug, Clone)]
enum Msg {
    Receive { content: String },
    Close { reason: Option<String> },
    Error { reason: String },
}

async fn watcher_in(path: &Path, state: Arc<SharedState>) -> Result<(), Error> {
    let close_state = state.clone();
    let watcher = Watchexec::new(move |mut handler| {
        if handler.signals().any(|signal| signal == Signal::Interrupt) {
            if let Err(reason) = close_state.close.send(()) {
                eprintln!("Could not close channel gracefully; hard closing it.\n{reason}");
                handler.quit();
            }
        }

        handler
    })
    .map_err(Error::from)?;

    watcher.config.pathset([path]);
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
