use crate::error::ServiceError;
use brainiac_core::database::{
    BrainiacDbError,
    connection::{BrainiacDbPool, BrainiacDbPoolConnection},
};

pub async fn run_brainiac_blocking<F, T>(pool: BrainiacDbPool, f: F) -> Result<T, ServiceError>
where
    F: FnOnce(&mut BrainiacDbPoolConnection) -> Result<T, ServiceError> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || -> Result<T, ServiceError> {
        let mut conn = pool.get().map_err(BrainiacDbError::from)?;
        f(&mut conn)
    })
    .await?
}
