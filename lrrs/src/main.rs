use miette::{IntoDiagnostic, Result};

use lrrs::live_reload;

#[tokio::main]
async fn main() -> Result<()> {
    // Skip the program name.
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    println!("Watching {:?}", &args);
    live_reload(&args).await.into_diagnostic()
}
