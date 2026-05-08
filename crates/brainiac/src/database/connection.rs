use crate::database::{BrainiacDbError, brainiac_database};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

pub type BrainiacDbPool = Pool<SqliteConnectionManager>;
pub type BrainiacDbPoolConnection = PooledConnection<SqliteConnectionManager>;

const BRAINIAC_MIGRATIONS: &[&str] = &[include_str!("migrations/initialize_tables.sql")];

fn run_migrations(conn: BrainiacDbPoolConnection) -> rusqlite::Result<()> {
    for migration in BRAINIAC_MIGRATIONS {
        conn.execute_batch(migration)?;
    }
    Ok(())
}

pub fn pool() -> Result<BrainiacDbPool, BrainiacDbError> {
    let db_path = brainiac_database();
    let is_new = !db_path.exists();
    let manager = SqliteConnectionManager::file(db_path).with_init(|c| {
        c.execute_batch("PRAGMA foreign_keys = ON; PRAGMA recursive_triggers = ON;")
    });
    let pool = r2d2::Pool::new(manager)?;
    if is_new {
        log::info!("New database detected, running migrations");
        run_migrations(pool.get()?)?;
        log::info!("Migrations completed successfully");
    }
    Ok(pool)
}
