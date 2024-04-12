use std::path::PathBuf;

use live_reload::serve;

fn main() -> Result<(), String> {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .and_then(|p| if p.is_dir() { Some(p) } else { None })
        .expect("the first argument should be a directory");

    serve(dir)
}
