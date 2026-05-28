use crate::database::{BrainiacDbError, brainiac_database};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

pub type BrainiacDbPool = Pool<SqliteConnectionManager>;
pub type BrainiacDbPoolConnection = PooledConnection<SqliteConnectionManager>;

const BRAINIAC_MIGRATIONS: &[&str] = &[include_str!("migrations/initialize_tables.sql")];

fn run_migrations(conn: &BrainiacDbPoolConnection) -> rusqlite::Result<()> {
    for (i, migration) in BRAINIAC_MIGRATIONS.iter().enumerate() {
        log::debug!("brainiac: running migration {}/{}", i + 1, BRAINIAC_MIGRATIONS.len());
        conn.execute_batch(migration).inspect_err(|e| {
            log::error!("brainiac: migration {}/{} failed: {e}", i + 1, BRAINIAC_MIGRATIONS.len());
        })?;
    }
    log::debug!("brainiac: all migrations applied");
    Ok(())
}

pub fn pool() -> Result<BrainiacDbPool, BrainiacDbError> {
    let db_path = brainiac_database();
    log::info!("brainiac: opening database at {}", db_path.display());
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
        log::error!("brainiac: failed to build connection pool for {}: {e}", db_path.display());
    })?;
    let conn = pool.get().inspect_err(|e| {
        log::error!("brainiac: failed to acquire initial connection from pool: {e}");
    })?;
    run_migrations(&conn)?;
    log::info!("brainiac: database ready");
    Ok(pool)
}
