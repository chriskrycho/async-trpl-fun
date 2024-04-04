use std::path::PathBuf;

use lr2::watch;

#[tokio::main]
async fn main() -> Result<(), lr2::Error> {
    let dir = std::env::args().nth(1).unwrap();
    let dir = PathBuf::from(dir);
    watch(&dir).await
}
