#[derive(Debug, thiserror::Error)]
pub enum BrainiacError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
    #[error(transparent)]
    Fsrs(#[from] fsrs::FSRSError),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
}
