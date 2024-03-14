use mini_redis::client;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> mini_redis::Result<()> {
    tokio::spawn(async { publish().await });

    subscribe().await?;

    println!("DONE");

    Ok(())
}

async fn subscribe() -> mini_redis::Result<()> {
    let client = client::connect("127.0.0.1:6379").await?;
    let subscriber = client.subscribe(vec!["numbers".to_string()]).await?;
    let messages = subscriber
        .into_stream()
        .filter_map(|msg| match msg {
            Ok(msg) if msg.content.len() == 1 => Some(msg.content),
            _ => None,
        })
        .take(3);

    tokio::pin!(messages);

    while let Some(msg) = messages.next().await {
        println!("got = {msg:?}");
    }

    // When the server closes, this will stop getting messages, and specifically
    // the `Future` will end so the above stream will end, by way of returning
    // `Poll::Ready(None)` via the implicit polling which happens in `.await`.
    // This `println!` is a handy little signal of just that in this demo.
    println!("(closed)");

    Ok(())
}

async fn publish() -> mini_redis::Result<()> {
    let mut client = client::connect("127.0.0.1:6379").await?;

    println!("starting publishing");
    client.publish("numbers", "1".into()).await?;
    println!("published '1'");
    client.publish("numbers", "two".into()).await?;
    println!("published 'two'");
    client.publish("numbers", "3".into()).await?;
    println!("published '3'");
    client.publish("numbers", "four".into()).await?;
    println!("published 'four'");
    client.publish("numbers", "five".into()).await?;
    println!("published 'five'");
    client.publish("numbers", "6".into()).await?;
    println!("published '6'");
    Ok(())
}
