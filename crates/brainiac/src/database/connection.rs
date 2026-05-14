use crate::database::{BrainiacDbError, brainiac_database};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

pub type BrainiacDbPool = Pool<SqliteConnectionManager>;
pub type BrainiacDbPoolConnection = PooledConnection<SqliteConnectionManager>;

const BRAINIAC_MIGRATIONS: &[&str] = &[include_str!("migrations/initialize_tables.sql")];

fn run_migrations(conn: &BrainiacDbPoolConnection) -> rusqlite::Result<()> {
    for migration in BRAINIAC_MIGRATIONS {
        conn.execute_batch(migration)?;
    }
    Ok(())
}

pub fn pool() -> Result<BrainiacDbPool, BrainiacDbError> {
    let manager = SqliteConnectionManager::file(brainiac_database()).with_init(|c| {
        c.execute_batch("PRAGMA foreign_keys = ON; PRAGMA recursive_triggers = ON;")
    });
    let pool = r2d2::Pool::new(manager)?;
    run_migrations(&pool.get()?)?;
    Ok(pool)
}
