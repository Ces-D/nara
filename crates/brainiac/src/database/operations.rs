use chrono::{DateTime, Utc};

use crate::database::{
    connection::BrainiacDbPool,
    models::{
        Category, CategoryTag, CategoryTagLink, CreateCategory, CreateItem, Item, ItemState,
        PracticeItem, PracticeItemAnswer, Rating, UpdateItem,
    },
};
use crate::error::BrainiacError;
use crate::scheduling::Scheduler;

/// Decodes an epoch-seconds column into a `DateTime<Utc>`, panicking only on a
/// value that can't represent a valid timestamp (which our schema never stores).
fn ts(secs: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(secs, 0).expect("column is not a valid epoch-seconds timestamp")
}

/////////////// Category

/// Inserts a new category and returns its row id.
pub async fn create_category(
    pool: &BrainiacDbPool,
    category: CreateCategory,
) -> Result<i64, BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let created_at = category.created_at.unwrap_or_else(Utc::now).timestamp();
        conn.execute(
            "INSERT INTO category (name, description, created_at) VALUES (?1, ?2, ?3)",
            (&category.name, &category.description, created_at),
        )?;
        Ok::<_, BrainiacError>(conn.last_insert_rowid())
    })
    .await?
}

/// Deletes a category by id; cascades to its items, item states, and tag links.
/// Returns `true` if a row was removed.
pub async fn delete_category(pool: &BrainiacDbPool, id: i64) -> Result<bool, BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        conn.execute("DELETE FROM category WHERE id = ?1", [id])?;
        Ok::<_, BrainiacError>(conn.changes() > 0)
    })
    .await?
}

/// Returns all categories paired with their tags, including each category's most recent practice timestamp.
pub async fn list_categories_with_tags(
    pool: &BrainiacDbPool,
) -> Result<Vec<(Category, Vec<CategoryTag>)>, BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut categories: Vec<(Category, Vec<CategoryTag>)> = conn
            .prepare(
                "SELECT c.id, c.name, c.description, c.created_at, MAX(s.last_reviewed_at) \
                 FROM category c \
                 LEFT JOIN item i ON i.category_id = c.id \
                 LEFT JOIN item_state s ON s.item_id = i.id \
                 GROUP BY c.id \
                 ORDER BY c.id",
            )?
            .query_map([], |row| {
                Ok((
                    Category {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        created_at: ts(row.get(3)?),
                        last_practiced: row.get::<_, Option<i64>>(4)?.map(ts),
                    },
                    Vec::<CategoryTag>::new(),
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let id_to_idx: std::collections::HashMap<i64, usize> = categories
            .iter()
            .enumerate()
            .map(|(i, (c, _))| (c.id, i))
            .collect();

        conn.prepare(
            "SELECT category_id, tag_name FROM category_tag_link ORDER BY category_id, tag_name",
        )?
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .for_each(|(category_id, tag_name)| {
            if let Some(&idx) = id_to_idx.get(&category_id) {
                categories[idx].1.push(CategoryTag { name: tag_name });
            }
        });

        Ok::<_, BrainiacError>(categories)
    })
    .await?
}

/////////////// Category Tag

/// Normalizes a tag (trim + uppercase) so reads and writes match.
fn prepare_tag(tag_name: String) -> String {
    tag_name.trim().to_uppercase()
}

/// Attaches a tag to a category, creating the tag row if needed. Idempotent.
pub async fn add_tag_to_category(
    pool: &BrainiacDbPool,
    tag: CategoryTagLink,
) -> Result<(), BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let tx = conn.transaction()?;
        let tag_name = prepare_tag(tag.tag_name);
        tx.execute(
            "INSERT OR IGNORE INTO category_tag (name) VALUES (?1)",
            [&tag_name],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO category_tag_link (category_id, tag_name) VALUES (?1, ?2)",
            (tag.category_id, &tag_name),
        )?;
        tx.commit()?;
        Ok::<_, BrainiacError>(())
    })
    .await?
}

/// Detaches a tag from a category. Orphaned tags are cleaned up by a DB trigger.
/// Returns `true` if a link was removed.
pub async fn remove_tag_from_category(
    pool: &BrainiacDbPool,
    category_id: i64,
    tag: String,
) -> Result<bool, BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let tag_name = prepare_tag(tag);
        conn.execute(
            "DELETE FROM category_tag_link WHERE category_id = ?1 AND tag_name = ?2",
            (category_id, &tag_name),
        )?;
        Ok::<_, BrainiacError>(conn.changes() > 0)
    })
    .await?
}

pub async fn list_tags(pool: &BrainiacDbPool) -> Result<Vec<CategoryTag>, BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let tags = conn
            .prepare("SELECT name FROM category_tag ORDER BY name")?
            .query_map([], |row| Ok(CategoryTag { name: row.get(0)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok::<_, BrainiacError>(tags)
    })
    .await?
}

/////////////// Item

/// Inserts items and their initial `item_state` rows in a single transaction.
pub async fn create_items(
    pool: &BrainiacDbPool,
    items: Vec<CreateItem>,
) -> Result<(), BrainiacError> {
    if items.is_empty() {
        return Ok(());
    }
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let now = Utc::now();
        let mut conn = pool.get()?;
        let tx = conn.transaction()?;
        {
            let mut item_stmt = tx.prepare(
                "INSERT INTO item (category_id, front, back, created_at) VALUES (?1, ?2, ?3, ?4)",
            )?;
            let mut state_stmt = tx.prepare(
                "INSERT INTO item_state (item_id, stability, difficulty, due_at, last_reviewed_at, reps, lapses) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;

            for item in items {
                item_stmt.execute((
                    item.category_id,
                    &item.front,
                    &item.back,
                    item.created_at.unwrap_or(now).timestamp(),
                ))?;
                let item_id = tx.last_insert_rowid();
                state_stmt.execute((
                    item_id,
                    Option::<f64>::None,
                    Option::<f64>::None,
                    now.timestamp(),
                    Option::<i64>::None,
                    0i32,
                    0i32,
                ))?;
            }
        }
        tx.commit()?;
        Ok::<_, BrainiacError>(())
    })
    .await?
}

/// Fetches one item by id. `last_reviewed_at` is left unset; callers needing it
/// should read `item_state` separately.
pub async fn get_item(pool: &BrainiacDbPool, item_id: i64) -> Result<Item, BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let item = conn.query_row(
            "SELECT id, category_id, front, back, created_at FROM item WHERE id = ?1",
            [item_id],
            |row| {
                Ok(Item {
                    id: row.get(0)?,
                    category_id: row.get(1)?,
                    front: row.get(2)?,
                    back: row.get(3)?,
                    created_at: ts(row.get(4)?),
                    last_reviewed_at: None,
                })
            },
        )?;
        Ok::<_, BrainiacError>(item)
    })
    .await?
}

/// Updates an item's front and/or back; `None` fields are left unchanged.
/// Returns `true` if a row was modified.
pub async fn update_item(
    pool: &BrainiacDbPool,
    item_id: i64,
    update: UpdateItem,
) -> Result<bool, BrainiacError> {
    if update.front.is_none() && update.back.is_none() {
        return Ok(false);
    }
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        conn.execute(
            "UPDATE item SET front = COALESCE(?1, front), back = COALESCE(?2, back) WHERE id = ?3",
            (&update.front, &update.back, item_id),
        )?;
        Ok::<_, BrainiacError>(conn.changes() > 0)
    })
    .await?
}

/// Deletes an item; cascades to its `item_state` row. Returns `true` if a row was removed.
pub async fn delete_item(pool: &BrainiacDbPool, item_id: i64) -> Result<bool, BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        conn.execute("DELETE FROM item WHERE id = ?1", [item_id])?;
        Ok::<_, BrainiacError>(conn.changes() > 0)
    })
    .await?
}

/////////////// Practice

/// Synchronous core of practice selection, shared by the public async fns so a
/// single pooled connection can serve both the item and its answer.
fn select_practice_items(
    conn: &crate::database::connection::BrainiacDbPoolConnection,
    limit: u8,
    category_ids: Option<Vec<i64>>,
    tag_names: Option<Vec<String>>,
) -> Result<Vec<PracticeItem>, BrainiacError> {
    let now = Utc::now().timestamp();

    let categories = category_ids.filter(|v| !v.is_empty());
    let tags = tag_names
        .map(|v| v.into_iter().map(prepare_tag).collect::<Vec<_>>())
        .filter(|v| !v.is_empty());

    let mut sql =
        String::from("SELECT i.id, i.front FROM item i JOIN item_state s ON s.item_id = i.id");
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut clauses: Vec<String> = Vec::new();

    if let Some(ids) = &categories {
        let placeholders = vec!["?"; ids.len()].join(", ");
        clauses.push(format!("i.category_id IN ({placeholders})"));
        for id in ids {
            params.push(Box::new(*id));
        }
    }
    if let Some(names) = &tags {
        let placeholders = vec!["?"; names.len()].join(", ");
        clauses.push(format!(
            "i.category_id IN (SELECT category_id FROM category_tag_link WHERE tag_name IN ({placeholders}))"
        ));
        for n in names {
            params.push(Box::new(n.clone()));
        }
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" OR "));
    }

    sql.push_str(
        " ORDER BY \
         CASE WHEN s.stability IS NULL THEN 0 ELSE 1 END ASC, \
         CAST(? - COALESCE(s.last_reviewed_at, 0) AS REAL) \
            / (COALESCE(s.stability, 1.0) * 86400.0) DESC \
         LIMIT ?",
    );
    params.push(Box::new(now));
    params.push(Box::new(limit as i64));

    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(PracticeItem {
                id: row.get(0)?,
                front: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(items)
}

/// Synchronous core for reading an item's answer, shared by the public async fns.
fn select_practice_item_answer(
    conn: &crate::database::connection::BrainiacDbPoolConnection,
    item_id: i64,
) -> Result<PracticeItemAnswer, BrainiacError> {
    let answer = conn.query_row(
        "SELECT id, back FROM item WHERE id = ?1",
        [item_id],
        |row| {
            Ok(PracticeItemAnswer {
                id: row.get(0)?,
                back: row.get(1)?,
            })
        },
    )?;
    Ok(answer)
}

/// Fetches up to `limit` items to practice, never-reviewed first then most overdue.
/// `category_ids` and `tag_names` are combined with OR; either may be `None`/empty.
pub async fn get_practice_items(
    pool: &BrainiacDbPool,
    limit: u8,
    category_ids: Option<Vec<i64>>,
    tag_names: Option<Vec<String>>,
) -> Result<Vec<PracticeItem>, BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        select_practice_items(&conn, limit, category_ids, tag_names)
    })
    .await?
}

/// Fetches the `back` of an item, used to reveal the answer during practice.
pub async fn get_practice_item_answer(
    pool: &BrainiacDbPool,
    item_id: i64,
) -> Result<PracticeItemAnswer, BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        select_practice_item_answer(&conn, item_id)
    })
    .await?
}

/// Fetches the next practice item together with its answer in a single
/// connection acquisition. Returns `None` when no item matches the filters.
pub async fn get_practice_item_with_answer(
    pool: &BrainiacDbPool,
    category_ids: Option<Vec<i64>>,
    tag_names: Option<Vec<String>>,
) -> Result<Option<(PracticeItem, PracticeItemAnswer)>, BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let Some(item) = select_practice_items(&conn, 1, category_ids, tag_names)?.pop() else {
            return Ok(None);
        };
        let answer = select_practice_item_answer(&conn, item.id)?;
        Ok::<_, BrainiacError>(Some((item, answer)))
    })
    .await?
}

/// Records a rating for an item: runs FSRS to schedule the next review and
/// writes the new stability, difficulty, due date, reps, and lapses.
/// Read and write happen in a single transaction so concurrent ratings on the
/// same item can't lose updates.
pub async fn rate_practice_item(
    pool: &BrainiacDbPool,
    item_id: i64,
    rating: Rating,
) -> Result<(), BrainiacError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let tx = conn.transaction()?;

        let current = tx.query_row(
            "SELECT item_id, stability, difficulty, due_at, last_reviewed_at, reps, lapses \
             FROM item_state WHERE item_id = ?1",
            [item_id],
            |row| {
                Ok(ItemState {
                    item_id: row.get(0)?,
                    stability: row.get(1)?,
                    difficulty: row.get(2)?,
                    due_at: ts(row.get(3)?),
                    last_reviewed_at: row.get::<_, Option<i64>>(4)?.map(ts),
                    reps: row.get(5)?,
                    lapses: row.get(6)?,
                })
            },
        )?;

        let now = Utc::now();
        let scheduler = Scheduler::new()?;
        let (memory, due_at) = scheduler.process_review(&current, rating, now)?;

        let learned = is_learned(current.reps, current.lapses);
        let reps = update_reps(rating, current.reps);
        let lapses = update_lapses(rating, current.lapses, learned);

        tx.execute(
            "UPDATE item_state SET stability = ?1, difficulty = ?2, due_at = ?3, last_reviewed_at = ?4, reps = ?5, lapses = ?6 WHERE item_id = ?7",
            (
                memory.stability as f64,
                memory.difficulty as f64,
                due_at.timestamp(),
                now.timestamp(),
                reps,
                lapses,
                item_id,
            ),
        )?;

        tx.commit()?;
        Ok::<_, BrainiacError>(())
    })
    .await?
}

/// Increments the rep counter on any successful recall (Hard, Good, or Easy).
fn update_reps(rating: Rating, current: u32) -> u32 {
    match rating {
        Rating::Hard | Rating::Good | Rating::Easy => current + 1,
        Rating::Again => current,
    }
}

/// Increments the lapse counter when a learned item is forgotten.
/// Only `Again` is a forgetting event; Hard is a difficult-but-successful recall.
/// Items still in the learning phase don't accrue lapses.
fn update_lapses(rating: Rating, current: u32, is_learned: bool) -> u32 {
    if is_learned && matches!(rating, Rating::Again) {
        current + 1
    } else {
        current
    }
}

/// An item is considered learned once it has been successfully recalled at least once.
fn is_learned(reps: u32, _lapses: u32) -> bool {
    reps > 0
}
