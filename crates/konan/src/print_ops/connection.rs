use crate::print_job_database;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

pub type KonanDbPool = Pool<SqliteConnectionManager>;
pub type KonanDbPoolConnection = PooledConnection<SqliteConnectionManager>;

#[derive(Debug, thiserror::Error)]
pub enum KonanDbError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
}

const KONAN_PULSE_MIGRATIONS: &[&str; 2] = &[
    include_str!("migrations/initialize_schedule_table.sql"),
    include_str!("migrations/initialize_print_job_table.sql"),
];

fn run_migrations(conn: &KonanDbPoolConnection) -> Result<(), KonanDbError> {
    for migration in KONAN_PULSE_MIGRATIONS {
        conn.execute_batch(migration)?;
    }
    Ok(())
}

pub fn pool() -> Result<KonanDbPool, KonanDbError> {
    let db_path = print_job_database();
    let manager = SqliteConnectionManager::file(db_path).with_init(|c| {
        c.execute_batch(
            "PRAGMA journal_mode = WAL;\
             PRAGMA synchronous = NORMAL;\
             PRAGMA busy_timeout = 5000;\
             PRAGMA foreign_keys = ON;\
             PRAGMA recursive_triggers = ON;",
        )
    });
    let pool = r2d2::Pool::new(manager)?;
    // Migrations are idempotent (CREATE TABLE IF NOT EXISTS), so run on every
    // startup. Any future non-idempotent migration will need a version table.
    run_migrations(&pool.get()?)?;
    Ok(pool)
}
