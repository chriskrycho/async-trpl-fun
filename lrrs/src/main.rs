use lrrs::serve;

fn main() -> Result<(), lrrs::Error> {
    // get the path to watch (skip the program name)
    let path = std::env::args().nth(1).unwrap();
    serve(path)
}
