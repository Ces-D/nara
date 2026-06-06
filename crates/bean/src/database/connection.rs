use std::path::PathBuf;

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

use crate::error::BeanError;

pub type BeanDBPool = Pool<SqliteConnectionManager>;
pub type BeanDBPoolConnection = PooledConnection<SqliteConnectionManager>;

const BEAN_MIGRATIONS: &[&str; 2] = &[
    include_str!("migrations/001_category.sql"),
    include_str!("migrations/002_entry.sql"),
];

fn run_migrations(conn: &BeanDBPoolConnection) -> Result<(), BeanError> {
    for (i, migration) in BEAN_MIGRATIONS.iter().enumerate() {
        log::debug!(
            "bean: running migration {}/{}",
            i + 1,
            BEAN_MIGRATIONS.len()
        );
        conn.execute_batch(migration).inspect_err(|e| {
            log::error!(
                "bean: migration {}/{} failed: {e}",
                i + 1,
                BEAN_MIGRATIONS.len()
            );
        })?;
    }
    log::debug!("bean: all migrations applied");
    Ok(())
}

fn database_loc() -> PathBuf {
    let home = std::env::home_dir().expect("Unable to find HOME env variable");
    let dir = home.join(".local/share/bean");
    if !dir.exists() {
        log::info!("bean: creating storage directory at {}", dir.display());
        std::fs::create_dir_all(&dir).unwrap_or_else(|_| {
            panic!(
                "Unable to create bean storage directory at: {}",
                dir.display()
            )
        });
    }
    dir.join("bean.db")
}

pub fn pool() -> Result<BeanDBPool, BeanError> {
    let db_path = database_loc();
    log::info!("bean: opening database at {}", db_path.display());
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
            "bean: failed to build connection pool for {}: {e}",
            db_path.display()
        );
    })?;
    let conn = pool.get().inspect_err(|e| {
        log::error!("bean: failed to acquire initial connection from pool: {e}");
    })?;
    run_migrations(&conn)?;
    log::info!("bean: database ready");
    Ok(pool)
}
