use std::{net::SocketAddr, path::Path, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket},
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
                token.cancel();
            }
        }
    });

    let (tx, mut rx) = broadcast::channel(100);
    let (close_tx, mut close_rx) = mpsc::unbounded_channel();
    let shared = Arc::new(SharedState {
        tx,
        close: close_tx,
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
                token.cancel();
            },

            // And now for the actual good part: handling the message loop!
            _ = async {
                loop {
                    match rx.recv().await {
                        Ok(Msg::Receive { content }) => println!("{content}"),
                        Ok(Msg::Reload) => println!("reload!"),
                        Ok(Msg::Close { reason }) => {
                            println!("close: {}", reason.unwrap_or("unknown".into()));
                        },
                        Ok(Msg::Error { reason }) => {
                            eprintln!("uh oh: {reason}");
                            break
                        }
                        Err(e) => {
                            eprintln!("uh oh: {e}");
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
        .route_service("/*asset", serve_dir)
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
    tx: broadcast::Sender<Msg>,
    close: mpsc::UnboundedSender<()>,
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<Arc<SharedState>>) -> Response {
    eprintln!("upgrading the websocket");
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Arc<SharedState>) {
    let (mut sender, mut receiver) = stream.split();
    let mut rx = state.tx.subscribe();

    loop {
        let (ws_reply, msg) = tokio::select! {
            message = receiver.next() => {
                let Some(message) = message else {
                    break;
                };

                eprintln!("got {message:?} from websocket");

                match message {
                    Ok(Message::Text(content)) => {
                        println!("Got a message!: '{content}'");
                        (None, Some(Msg::Receive { content }))
                    }

                    Ok(Message::Binary(_)) => {
                        (Some(Message::Text(String::from("Binary data is not supported"))), None)
                    }

                    Ok(Message::Ping(_) | Message::Pong(_)) => (None, None),

                    Ok(Message::Close(maybe_frame)) => {
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

                        (None, Some(message))
                    }

                    Err(reason) => {
                        let message = Msg::Error {
                            reason: reason.to_string()
                        };

                        (None, Some(message))
                    }
                }
            },
            msg = rx.recv() => match msg {
                Ok(Msg::Reload) => {
                    (Some(Message::Text(String::from("reload"))), None)
                },

                Ok(_) => (None, None),

                /* TODO: error handling! */
                Err(_) => (None, None),
            }
        };

        if let Some(reply) = ws_reply {
            eprint!("sending ws message…");
            sender.send(reply).await.unwrap(); // TODO: error handling!
            eprintln!(" done.");
        }

        if let Some(internal) = msg {
            eprint!("sending internal message…");
            state.tx.send(internal).unwrap(); // TODO: error handling!
            eprintln!(" done.");
        }
    }
}

#[derive(Debug, Clone)]
enum Msg {
    Receive { content: String },
    Reload,
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
            let should_reload = handler.events.iter().any(|event| event.paths().count() > 0);
            if should_reload {
                future_state.tx.send(Msg::Reload).unwrap();
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
