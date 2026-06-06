#[derive(Debug, thiserror::Error)]
pub enum BeanError {
    #[error("category not found")]
    CategoryNotFound,
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
}
