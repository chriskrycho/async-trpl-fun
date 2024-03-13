use std::pin::pin;
use futures::executor::block_on;

fn main() {
    let mut me_future = pin!(get_me());

    block_on(async {
        let val = (&mut me_future).await;
        println!("{val:?}");

        // This will panic. 🫨
        let val = (&mut me_future).await;
        println!("{val:?}");
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
