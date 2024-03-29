use std::path::PathBuf;

use lrrs::serve;

fn main() -> Result<(), lrrs::Error> {
    let path = std::env::args().nth(1).unwrap();
    serve(PathBuf::from(path))
}
