use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{SinkExt as _, StreamExt as _};
use notify::{recommended_watcher, RecursiveMode, Watcher};
use tokio::{
    net::TcpListener,
    runtime::Runtime,
    signal::ctrl_c,
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    task,
};
use tower_http::services::ServeDir;

pub fn serve(path: PathBuf) -> Result<(), String> {
    let rt = Runtime::new().map_err(|error| format!("{error}"))?;

    let (tx, rx) = mpsc::channel(10);
    let rx = Arc::new(Mutex::new(rx));

    let mut set = task::JoinSet::new();
    let server_handle = set.spawn_on(server_in(path.to_owned(), rx), rt.handle());
    let watcher_handle = set.spawn_on(watcher_in(path.to_owned(), tx), rt.handle());

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
        while let Some(result) = set.join_next().await {
            match result {
                Ok(Ok(_)) => {}
                Ok(Err(reason)) => return Err(format!("inner: {reason}")),
                Err(reason) => return Err(format!("outer: {reason}")),
            }
        }

        Ok(())
    })
}

async fn watcher_in(dir: PathBuf, event_channel: Sender<Change>) -> Result<(), String> {
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
                eprintln!("Got an event:\n\t{:?}\n\t{:?}", event.kind, event.source());
                let change = Change { paths: event.paths };
                if let Err(e) = event_channel.send(change).await {
                    eprintln!("Error sending out: {e:?}");
                }
            }
            Err(reason) => return Err(format!("{reason}")),
        }
    }

    Ok(())
}

type SharedRx = Arc<Mutex<Receiver<Change>>>;

#[derive(Debug, Clone)]
struct Change {
    pub paths: Vec<PathBuf>,
}

// I suspect we may want to abstract this a bit. Make *most* of this particular
// thing one of the things we supply via our managed crate. In that approach, we
// can basically *just* have the readers implement the WebSocket handler.
async fn server_in(path: PathBuf, state: SharedRx) -> Result<(), String> {
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

#[derive(Debug, Clone)]
struct WebSocketClosed {
    reason: Option<String>,
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(rx): State<SharedRx>) -> Response {
    eprintln!("upgrading the websocket");
    ws.on_upgrade(|socket| websocket(socket, rx))
}

async fn websocket(stream: WebSocket, rx: SharedRx) {
    let (mut ws_sender, mut receiver) = stream.split();

    let mut listening = true;

    // TODO: split into two tasks

    loop {
        if let Some(Change { paths }) = rx.lock().await.recv().await {
            // TODO: only reload specific paths.
            if listening {
                eprint!("sending WebSocket reload message…");
                ws_sender
                    .send(Message::Text(String::from("reload")))
                    .await
                    .unwrap(); // TODO: error handling!

                eprintln!(" done.");
            }
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

fn handle(message: Result<Message, axum::Error>) -> Result<Option<WebSocketClosed>, String> {
    eprintln!("got {message:?} from WebSocket");

    use Message::*;
    match message {
        Ok(message) => match message {
            // We don't care about *receiving* messages from the WebSocket, only
            // sending messages *to* it.
            Text(_) | Binary(_) | Ping(_) | Pong(_) => Ok(None),

            // We *do* care if the socket closes.
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

        Err(reason) => Err(format!("{reason}")),
    }
}
