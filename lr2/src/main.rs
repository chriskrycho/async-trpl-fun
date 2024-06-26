use std::path::PathBuf;

use lr2::watch;

#[tokio::main]
async fn main() -> Result<(), String> {
    let dir = std::env::args().nth(1).map(PathBuf::from).unwrap();
    let dir = PathBuf::from(dir);
    let (tx, mut rx) = tokio::sync::mpsc::channel(256);

    let mut set = tokio::task::JoinSet::new();

    set.spawn(watch(dir, tx));
    set.spawn(async move {
        while let Some(lr2::Change { paths }) = rx.recv().await {
            println!(
                "Got a change! Paths: {}",
                paths
                    .iter()
                    .map(|path| format!("{}", path.display()))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        Ok(())
    });

    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(_)) => {}
            Ok(Err(reason)) => return Err(format!("{reason}")),
            Err(reason) => return Err(format!("{reason}")),
        }
    }

    Ok(())
}
