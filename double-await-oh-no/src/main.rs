use std::pin::pin;
use futures::executor::block_on;

fn main() {
    let mut me_future = pin!(get_me());

    // This will panic!
    block_on(async {
        (&mut me_future).await;
        (&mut me_future).await;
    });
}

#[derive(Debug)]
struct Person {
    name: Option<&'static str>,
    age: u8,
}

async fn get_me() -> Person {
    Person {
        name: Some("Chris"),
        age: 36,
    }
}
