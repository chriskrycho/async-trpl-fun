use std::time::Duration;

use async_http_server::ThreadPool;
use tokio::{
    fs,
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time,
};
use tokio_stream::{
    wrappers::{LinesStream, TcpListenerStream},
    StreamExt,
};

#[tokio::main]
async fn main() {
    let mut listener = TcpListener::bind("127.0.0.1:7878")
        .await
        .unwrap()
        .to_stream();

    let pool = ThreadPool::new(4);

    while let Some(stream) = listener.next().await {
        let stream = stream.unwrap();
        pool.execute(async {
            println!("Executing task");
            handle_connection(stream).await;
        });
    }
}

async fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines_stream().next().await.unwrap().unwrap();

    let (status_line, file_name) = match request_line.as_str() {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            time::sleep(Duration::from_secs(5)).await;
            ("HTTP/1.1 200 OK", "hello.html")
        }
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let contents = fs::read_to_string(file_name).await.unwrap();
    let length = contents.len();

    let response =
        format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).await.unwrap();
}

trait ToListenerStream {
    fn to_stream(self) -> TcpListenerStream;
}

impl ToListenerStream for TcpListener {
    fn to_stream(self) -> TcpListenerStream {
        TcpListenerStream::new(self)
    }
}

// MAYBE: support code in `trpl` so people can just call `.lines_stream()` To
// make it work, we would need to have users import it *or* have them import a
// `prelude`, like `use trpl::{Something, prelude::*}`?
trait ToLinesStream<R> {
    fn lines_stream(self) -> LinesStream<R>;
}

impl<R> ToLinesStream<R> for R
where
    R: AsyncBufRead,
{
    fn lines_stream(self) -> LinesStream<R> {
        LinesStream::new(self.lines())
    }
}
