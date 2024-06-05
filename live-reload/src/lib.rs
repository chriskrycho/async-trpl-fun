use std::{net::SocketAddr, path::PathBuf, pin::pin};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{future, SinkExt, StreamExt};
use notify::{recommended_watcher, RecursiveMode, Watcher};
use tokio::{
    net::TcpListener,
    runtime::Runtime,
    signal::ctrl_c,
    sync::{
        broadcast::{self, error::RecvError, Sender},
        mpsc,
    },
    task,
};
use tower_http::services::ServeDir;

pub fn serve(path: PathBuf) -> Result<(), String> {
    let rt = Runtime::new().map_err(|error| format!("{error}"))?;

    // We only need the tx side, since we are going to take advantage of the
    // fact that it `broadcast::Sender` implements `Clone` to pass it around and
    // get easy and convenient access to local receivers with `tx.subscribe()`.
    let (tx, _rx) = broadcast::channel(10);

    let mut set = task::JoinSet::new();
    let server_handle = set.spawn_on(server_in(path.to_owned(), tx.clone()), rt.handle());
    let watcher_handle = set.spawn_on(watcher_in(path.to_owned(), tx.clone()), rt.handle());

    set.spawn_on(
        async move {
            ctrl_c().await.map_err(|error| format!("ctrl-c: {error}"))?;
            server_handle.abort();
            watcher_handle.abort();
            Ok(())
        },
        rt.handle(),
    );

    rt.block_on(async {
        eprintln!("Starting up `block_on` in `serve`.");
        while let Some(result) = set.join_next().await {
            match result {
                Ok(Ok(_)) => {
                    eprintln!("everything was awesome.")
                }
                Ok(Err(reason)) => return Err(format!("inner: {reason}")),
                Err(reason) => return Err(format!("outer: {reason}")),
            }
        }

        Ok(())
    })
}

async fn watcher_in(dir: PathBuf, change_tx: Tx) -> Result<(), String> {
    let (tx, mut rx) = mpsc::channel(256);

    // Doing this here means we will not drop the watcher until this function
    // ends, and the `while let` below will continue until there is an error (or
    // something else shuts down the whole system here!).
    let mut watcher = recommended_watcher(move |result| {
        if let Err(e) = tx.blocking_send(result) {
            eprintln!("Could not send event.\nError:{e}");
        }
    })
    .map_err(|error| format!("{error}"))?;

    watcher
        .watch(&dir, RecursiveMode::Recursive)
        .map_err(|error| format!("{error}"))?;

    while let Some(result) = rx.recv().await {
        match result {
            Ok(event) => {
                eprintln!("---");
                eprintln!("Got an event:\n\t{:?}\n\t{:?}", event.kind, event.source());
                let change = Change { paths: event.paths };
                if let Err(e) = change_tx.send(change) {
                    eprintln!("Error sending out: {e:?}");
                }
            }
            Err(reason) => return Err(format!("Other error: {reason}")),
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct Change {
    pub paths: Vec<PathBuf>,
}

/// Shorthand for typing!
type Tx = Sender<Change>;

// I suspect we may want to abstract this a bit. Make *most* of this particular
// thing one of the things we supply via our managed crate. In that approach, we
// can basically *just* have the readers implement the WebSocket handler.
async fn server_in(path: PathBuf, state: Tx) -> Result<(), String> {
    let serve_dir = ServeDir::new(path).append_index_html_on_directories(true);

    let router = Router::new()
        .nest_service("/", serve_dir)
        .route("/lr", get(ws_upgrade))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 9876));
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|error| format!("{error}"))?;

    eprintln!("→ Serving at: http://{addr}");

    axum::serve(listener, router)
        .await
        .map_err(|error| format!("{error}"))
}
async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<Tx>) -> Response {
    eprintln!("binding websocket upgrade");
    ws.on_upgrade(|socket| {
        eprintln!("upgrading the websocket");
        websocket(socket, state)
    })
}

async fn websocket(stream: WebSocket, change_tx: Tx) {
    let (mut ws_tx, mut ws_rx) = stream.split();
    let mut change_rx = change_tx.subscribe();

    let reload_fut = pin!(async {
        loop {
            match change_rx.recv().await {
                Ok(Change { paths: _paths }) => {
                    // TODO: only reload specific paths.
                    eprintln!("sending WebSocket reload message…");
                    match ws_tx.send(Message::Text(String::from("reload"))).await {
                        Ok(_) => println!("\tSent!"),
                        Err(reason) => eprintln!("\tError with reload: {reason}"),
                    }
                }
                Err(recv_error) => match recv_error {
                    RecvError::Closed => break,
                    RecvError::Lagged(skipped) => {
                        eprintln!("Lost {skipped} messages");
                    }
                },
            }
        }
    });

    let close_fut = pin!(async {
        while let Some(message) = ws_rx.next().await {
            match handle(message) {
                Ok(state) => match state {
                    WSState::Open => {
                        eprintln!("ws open, continuing")
                    }
                    WSState::Closed { reason } => {
                        eprintln!("ws closed ({reason:?}), breaking");
                        break;
                    }
                },
                Err(reason) => {
                    eprintln!("ws error: {reason}");
                    break;
                }
            }
        }
    });

    future::select(reload_fut, close_fut).await;
}

enum WSState {
    Open,
    Closed { reason: Option<String> },
}

fn handle(message: Result<Message, axum::Error>) -> Result<WSState, String> {
    eprintln!("got {message:?} from WebSocket");

    use Message::*;
    match message {
        Ok(message) => match message {
            // We don't care about *receiving* messages from the WebSocket, only
            // sending messages *to* it.
            Text(_) | Binary(_) => {
                Err("Unexpected message (this is a one-way conversation!".to_string())
            }
            Ping(_) | Pong(_) => {
                eprintln!("ping/ping");
                Ok(WSState::Open)
            }

            // We *do* care if the socket closes.
            Close(maybe_frame) => {
                let message = WSState::Closed {
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

                Ok(message)
            }
        },

        Err(reason) => Err(format!("{reason}")),
    }
}
