use crate::error::ServiceError;
use brainiac_core::database::{
    BrainiacDbError,
    connection::{BrainiacDbPool, BrainiacDbPoolConnection},
};
use konan_core::print_ops::{KonanDbError, KonanDbPool, KonanDbPoolConnection};

pub async fn run_konan_blocking<F, T>(pool: KonanDbPool, f: F) -> Result<T, ServiceError>
where
    F: FnOnce(&mut KonanDbPoolConnection) -> Result<T, ServiceError> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || -> Result<T, ServiceError> {
        let mut conn = pool.get().map_err(KonanDbError::from)?;
        f(&mut conn)
    })
    .await?
}

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
