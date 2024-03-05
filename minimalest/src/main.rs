use futures::executor::block_on;

fn main() {
    let result = block_on(hello());
    println!("{result}");
}

async fn hello() -> &'static str {
    "Hello, world!"
}
