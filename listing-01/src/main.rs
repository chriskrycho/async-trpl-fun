use futures::executor;

fn main() {
    executor::block_on(hello_async());
}

async fn hello_async() {
    println!("Hello from async!");
}
