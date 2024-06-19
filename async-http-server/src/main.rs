use tokio::{
    fs,
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
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

    let http_request: Vec<_> = buf_reader
        .lines_stream()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect()
        .await;

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("hello.html").await.unwrap();
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
