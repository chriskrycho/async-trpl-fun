use std::path::Path;

pub async fn live_reload(sources: &[impl AsRef<Path>]) -> Result<(), Error> {
    todo!()
}

#[derive(Debug, thiserror::Error)]
pub enum Error {}
