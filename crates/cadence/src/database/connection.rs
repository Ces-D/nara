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
    for migration in CADENCE_MIGRATIONS {
        conn.execute_batch(migration)?;
    }
    Ok(())
}

fn database_loc() -> PathBuf {
    let home = std::env::home_dir().expect("Unable to find HOME env variable");
    let dir = home.join(".local/share/cadence");
    if !dir.exists() {
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
    run_migrations(&pool.get()?)?;
    Ok(pool)
}
