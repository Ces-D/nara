use crate::error::BeanError;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

mod connection;
pub use connection::{BeanDBPool, BeanDBPoolConnection, pool};

/// A grouping of entries. Each category carries a required `description`
/// explaining the kind of information it holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub description: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
}

/// A piece of text information belonging to a category and a date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: i64,
    pub category_id: i64,
    /// One sentence summary of the content.
    pub name: String,
    pub content: String,
    /// The date the information belongs to.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub entry_date: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
}

// ~~~~~~~~~~~~ category ops

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategory {
    pub name: String,
    pub description: String,
}

pub async fn create_category(
    pool: &connection::BeanDBPool,
    category: CreateCategory,
) -> Result<i64, BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let now = Utc::now().timestamp();
        let conn = pool.get()?;
        conn.execute(
            "INSERT INTO category (name, description, created_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![category.name, category.description, now],
        )?;
        Ok::<_, BeanError>(conn.last_insert_rowid())
    })
    .await?
}

pub async fn get_category(
    pool: &connection::BeanDBPool,
    id: i64,
) -> Result<Option<Category>, BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let category = conn
            .query_row(
                "SELECT id, name, description, created_at FROM category WHERE id = ?1",
                rusqlite::params![id],
                row_to_category,
            )
            .optional()?;
        Ok::<_, BeanError>(category)
    })
    .await?
}

pub async fn get_category_by_name(
    pool: &connection::BeanDBPool,
    name: String,
) -> Result<Option<Category>, BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let category = conn
            .query_row(
                "SELECT id, name, description, created_at FROM category WHERE name = ?1",
                rusqlite::params![name],
                row_to_category,
            )
            .optional()?;
        Ok::<_, BeanError>(category)
    })
    .await?
}

pub async fn list_categories(pool: &connection::BeanDBPool) -> Result<Vec<Category>, BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt =
            conn.prepare("SELECT id, name, description, created_at FROM category ORDER BY name")?;
        let categories = stmt
            .query_map([], row_to_category)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, BeanError>(categories)
    })
    .await?
}

pub async fn update_category(
    pool: &connection::BeanDBPool,
    id: i64,
    update: CreateCategory,
) -> Result<(), BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let affected = conn.execute(
            "UPDATE category SET name = ?1, description = ?2 WHERE id = ?3",
            rusqlite::params![update.name, update.description, id],
        )?;
        if affected == 0 {
            return Err(BeanError::CategoryNotFound);
        }
        Ok::<_, BeanError>(())
    })
    .await?
}

pub async fn delete_category(pool: &connection::BeanDBPool, id: i64) -> Result<(), BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        conn.execute("DELETE FROM category WHERE id = ?1", rusqlite::params![id])?;
        Ok::<_, BeanError>(())
    })
    .await?
}

// ~~~~~~~~~~~~ entry ops

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntry {
    pub category_id: i64,
    pub name: String,
    pub content: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub entry_date: DateTime<Utc>,
}

pub async fn create_entry(
    pool: &connection::BeanDBPool,
    entry: CreateEntry,
) -> Result<i64, BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let now = Utc::now().timestamp();
        let conn = pool.get()?;
        // Guard the FK with a clearer error than a raw constraint violation.
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM category WHERE id = ?1)",
            rusqlite::params![entry.category_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(BeanError::CategoryNotFound);
        }
        conn.execute(
            "INSERT INTO entry (category_id, name, content, entry_date, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                entry.category_id,
                entry.name,
                entry.content,
                entry.entry_date.timestamp(),
                now,
            ],
        )?;
        Ok::<_, BeanError>(conn.last_insert_rowid())
    })
    .await?
}

pub async fn get_entry(pool: &connection::BeanDBPool, id: i64) -> Result<Option<Entry>, BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let entry = conn
            .query_row(
                "SELECT id, category_id, name, content, entry_date, created_at
                 FROM entry WHERE id = ?1",
                rusqlite::params![id],
                row_to_entry,
            )
            .optional()?;
        Ok::<_, BeanError>(entry)
    })
    .await?
}

#[derive(Debug, Default, Clone)]
pub struct ListEntriesFilter {
    pub category_id: Option<i64>,
    /// Inclusive lower bound on `entry_date`.
    pub from: Option<DateTime<Utc>>,
    /// Inclusive upper bound on `entry_date`.
    pub to: Option<DateTime<Utc>>,
}

pub async fn list_entries(
    pool: &connection::BeanDBPool,
    filter: ListEntriesFilter,
) -> Result<Vec<Entry>, BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, category_id, name, content, entry_date, created_at
             FROM entry
             WHERE (:category_id IS NULL OR category_id = :category_id)
               AND (:from IS NULL OR entry_date >= :from)
               AND (:to IS NULL OR entry_date <= :to)
             ORDER BY entry_date DESC, id DESC",
        )?;
        let entries = stmt
            .query_map(
                rusqlite::named_params! {
                    ":category_id": filter.category_id,
                    ":from": filter.from.map(|d| d.timestamp()),
                    ":to": filter.to.map(|d| d.timestamp()),
                },
                row_to_entry,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, BeanError>(entries)
    })
    .await?
}

pub async fn update_entry(
    pool: &connection::BeanDBPool,
    id: i64,
    update: CreateEntry,
) -> Result<(), BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let affected = conn.execute(
            "UPDATE entry
             SET category_id = ?1, name = ?2, content = ?3, entry_date = ?4
             WHERE id = ?5",
            rusqlite::params![
                update.category_id,
                update.name,
                update.content,
                update.entry_date.timestamp(),
                id,
            ],
        )?;
        if affected == 0 {
            return Err(BeanError::CategoryNotFound);
        }
        Ok::<_, BeanError>(())
    })
    .await?
}

pub async fn delete_entry(pool: &connection::BeanDBPool, id: i64) -> Result<(), BeanError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        conn.execute("DELETE FROM entry WHERE id = ?1", rusqlite::params![id])?;
        Ok::<_, BeanError>(())
    })
    .await?
}

// ~~~~~~~~~~~~ helpers

fn ts(row: &rusqlite::Row<'_>, col: &str) -> rusqlite::Result<DateTime<Utc>> {
    let v: i64 = row.get(col)?;
    DateTime::<Utc>::from_timestamp(v, 0).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            format!("invalid timestamp in {col}: {v}").into(),
        )
    })
}

fn row_to_category(row: &rusqlite::Row<'_>) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        created_at: ts(row, "created_at")?,
    })
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: row.get("id")?,
        category_id: row.get("category_id")?,
        name: row.get("name")?,
        content: row.get("content")?,
        entry_date: ts(row, "entry_date")?,
        created_at: ts(row, "created_at")?,
    })
}
