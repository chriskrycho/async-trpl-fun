use std::{thread, time::Duration};

use tokio::{
    fs,
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time,
};
use tokio_stream::{wrappers::LinesStream, StreamExt};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();

    loop {
        let (stream, _address) = listener.accept().await.unwrap();
        handle_connection(stream).await;
    }
}

async fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines_stream().next().await.unwrap().unwrap();

    let (status_line, file_name) = match request_line.as_str() {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            // Notice that this still blocks other requests. Just using async
            // is not a get-out-of-blocking-free card. The top-level `loop` does
            // not take advantage of async, but handles each request in series.
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
