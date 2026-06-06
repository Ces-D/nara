use std::path::PathBuf;

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

use crate::error::CadenceError;

pub type CadenceDBPool = Pool<SqliteConnectionManager>;
pub type CadenceDBPoolConnection = PooledConnection<SqliteConnectionManager>;

const CADENCE_MIGRATIONS: &[&str; 2] = &[
    include_str!("migrations/001_schedule.sql"),
    include_str!("migrations/002_job.sql"),
];

fn run_migrations(conn: &CadenceDBPoolConnection) -> Result<(), CadenceError> {
    for (i, migration) in CADENCE_MIGRATIONS.iter().enumerate() {
        log::debug!(
            "cadence: running migration {}/{}",
            i + 1,
            CADENCE_MIGRATIONS.len()
        );
        conn.execute_batch(migration).inspect_err(|e| {
            log::error!(
                "cadence: migration {}/{} failed: {e}",
                i + 1,
                CADENCE_MIGRATIONS.len()
            );
        })?;
    }
    log::debug!("cadence: all migrations applied");
    Ok(())
}

fn database_loc() -> PathBuf {
    let home = std::env::home_dir().expect("Unable to find HOME env variable");
    let dir = home.join(".local/share/cadence");
    if !dir.exists() {
        log::info!("cadence: creating storage directory at {}", dir.display());
        std::fs::create_dir_all(&dir).unwrap_or_else(|_| {
            panic!(
                "Unable to create cadence storage directory at: {}",
                dir.display()
            )
        });
    }
    dir.join("cadence.db")
}

pub fn pool() -> Result<CadenceDBPool, CadenceError> {
    let db_path = database_loc();
    log::info!("cadence: opening database at {}", db_path.display());
    let manager = SqliteConnectionManager::file(&db_path).with_init(|c| {
        c.execute_batch(
            "PRAGMA busy_timeout = 5000;\
             PRAGMA journal_mode = WAL;\
             PRAGMA synchronous = NORMAL;\
             PRAGMA foreign_keys = ON;\
             PRAGMA recursive_triggers = ON;",
        )
    });
    let pool = r2d2::Pool::new(manager).inspect_err(|e| {
        log::error!(
            "cadence: failed to build connection pool for {}: {e}",
            db_path.display()
        );
    })?;
    let conn = pool.get().inspect_err(|e| {
        log::error!("cadence: failed to acquire initial connection from pool: {e}");
    })?;
    run_migrations(&conn)?;
    log::info!("cadence: database ready");
    Ok(pool)
}
