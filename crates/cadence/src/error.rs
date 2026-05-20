#[derive(Debug, thiserror::Error)]
pub enum CadenceError {
    #[error("no channel registered")]
    NoChannel,
    #[error("artifact not accepted by channel")]
    ArtifactNotAccepted,
    #[error("channel: {0}")]
    Channel(String),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
}
