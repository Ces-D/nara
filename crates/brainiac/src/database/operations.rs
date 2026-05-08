use chrono::{DateTime, Utc};

use crate::database::{
    BrainiacDbError,
    connection::BrainiacDbPoolConnection,
    models::{
        Category, CategoryTag, CreateCategory, CreateItem, Item, ItemState, PracticeItem,
        PracticeItemAnswer, Rating, UpdateItem,
    },
};
use crate::scheduling::Scheduler;

/////////////// Category

/// Inserts a new category and returns its row id.
pub fn create_category(
    category: CreateCategory,
    conn: &BrainiacDbPoolConnection,
) -> Result<i64, BrainiacDbError> {
    let created_at = category
        .created_at
        .unwrap_or_else(chrono::Utc::now)
        .timestamp();
    conn.execute(
        "INSERT INTO category (name, description, created_at) VALUES (?1, ?2, ?3)",
        (&category.name, &category.description, created_at),
    )?;
    Ok(conn.last_insert_rowid())
}
/// Deletes a category by id; cascades to its items, item states, and tag links.
pub fn delete_category(id: i64, conn: &BrainiacDbPoolConnection) -> Result<(), BrainiacDbError> {
    conn.execute("DELETE FROM category WHERE id = ?1", [id])?;
    Ok(())
}
/// Returns all categories paired with their tags, including each category's most recent practice timestamp.
pub fn list_categories_with_tags(
    conn: &BrainiacDbPoolConnection,
) -> Result<Vec<(Category, Vec<CategoryTag>)>, BrainiacDbError> {
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
                    created_at: DateTime::from_timestamp(row.get(3)?, 0)
                        .expect("category.created_at is not a valid timestamp"),
                    last_practiced: row
                        .get::<_, Option<i64>>(4)?
                        .and_then(|s| DateTime::from_timestamp(s, 0)),
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

    Ok(categories)
}

/////////////// Category Tag

/// Normalizes a tag (trim + uppercase) so reads and writes match.
fn prepare_tag(tag_name: String) -> String {
    tag_name.trim().to_uppercase()
}

/// Attaches a tag to a category, creating the tag row if needed. Idempotent.
pub fn add_tag_to_category(
    category_id: i64,
    tag: String,
    conn: &mut BrainiacDbPoolConnection,
) -> Result<(), BrainiacDbError> {
    let tx = conn.transaction()?;
    let tag_name = prepare_tag(tag);
    tx.execute(
        "INSERT OR IGNORE INTO category_tag (name) VALUES (?1)",
        [&tag_name],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO category_tag_link (category_id, tag_name) VALUES (?1, ?2)",
        (category_id, &tag_name),
    )?;
    tx.commit()?;
    Ok(())
}

/// Detaches a tag from a category. Orphaned tags are cleaned up by a DB trigger.
pub fn remove_tag_from_category(
    category_id: i64,
    tag: String,
    conn: &BrainiacDbPoolConnection,
) -> Result<(), BrainiacDbError> {
    let tag_name = prepare_tag(tag);
    conn.execute(
        "DELETE FROM category_tag_link WHERE category_id = ?1 AND tag_name = ?2",
        (category_id, &tag_name),
    )?;
    Ok(())
}

pub fn list_tags(conn: &BrainiacDbPoolConnection) -> Result<Vec<CategoryTag>, BrainiacDbError> {
    conn.prepare("SELECT name FROM category_tag ORDER BY name")?
        .query_map([], |row| Ok(CategoryTag { name: row.get(0)? }))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(BrainiacDbError::from)
}

/////////////// Item

/// Inserts items and their initial `item_state` rows in a single transaction.
pub fn create_items(
    items: Vec<CreateItem>,
    conn: &mut BrainiacDbPoolConnection,
) -> Result<(), BrainiacDbError> {
    if items.is_empty() {
        return Ok(());
    }
    let now = Utc::now();
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
    Ok(())
}

/// Fetches one item by id. `last_reviewed_at` is left unset; callers needing it
/// should read `item_state` separately.
pub fn get_item(
    item_id: i64,
    conn: &BrainiacDbPoolConnection,
) -> Result<Item, BrainiacDbError> {
    let item = conn.query_row(
        "SELECT id, category_id, front, back, created_at FROM item WHERE id = ?1",
        [item_id],
        |row| {
            Ok(Item {
                id: row.get(0)?,
                category_id: row.get(1)?,
                front: row.get(2)?,
                back: row.get(3)?,
                created_at: DateTime::from_timestamp(row.get(4)?, 0)
                    .expect("item.created_at is not a valid timestamp"),
                last_reviewed_at: None,
            })
        },
    )?;
    Ok(item)
}

/// Updates an item's front and/or back; `None` fields are left unchanged.
pub fn update_item(
    item_id: i64,
    update: UpdateItem,
    conn: &BrainiacDbPoolConnection,
) -> Result<(), BrainiacDbError> {
    if update.front.is_none() && update.back.is_none() {
        return Ok(());
    }
    conn.execute(
        "UPDATE item SET front = COALESCE(?1, front), back = COALESCE(?2, back) WHERE id = ?3",
        (&update.front, &update.back, item_id),
    )?;
    Ok(())
}

/// Deletes an item; cascades to its `item_state` row.
pub fn delete_item(item_id: i64, conn: &BrainiacDbPoolConnection) -> Result<(), BrainiacDbError> {
    conn.execute("DELETE FROM item WHERE id = ?1", [item_id])?;
    Ok(())
}

/////////////// Item

/// Fetches up to `limit` items to practice, never-reviewed first then most overdue.
/// `category_ids` and `tag_names` are combined with OR; either may be `None`/empty.
pub fn get_practice_items(
    limit: u8,
    category_ids: Option<Vec<i64>>,
    tag_names: Option<Vec<String>>,
    conn: &BrainiacDbPoolConnection,
) -> Result<Vec<PracticeItem>, BrainiacDbError> {
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

/// Fetches the `back` of an item, used to reveal the answer during practice.
pub fn get_practice_item_answer(
    item_id: i64,
    conn: &BrainiacDbPoolConnection,
) -> Result<PracticeItemAnswer, BrainiacDbError> {
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

/// Records a rating for an item: runs FSRS to schedule the next review and
/// writes the new stability, difficulty, due date, reps, and lapses.
pub fn rate_practice_item(
    item_id: i64,
    rating: Rating,
    conn: &BrainiacDbPoolConnection,
) -> Result<(), BrainiacDbError> {
    let current = get_item_state(item_id, conn)?;
    let now = Utc::now();

    let scheduler = Scheduler::new()?;
    let (memory, due_at) = scheduler.process_review(&current, rating, now)?;

    let learned = is_learned(current.reps, current.lapses);
    let reps = update_reps(rating, current.reps);
    let lapses = update_lapses(rating, current.lapses, learned);

    conn.execute(
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
    Ok(())
}

/// Loads the FSRS scheduling state for one item.
fn get_item_state(
    item_id: i64,
    conn: &BrainiacDbPoolConnection,
) -> Result<ItemState, BrainiacDbError> {
    let state = conn.query_row(
        "SELECT item_id, stability, difficulty, due_at, last_reviewed_at, reps, lapses \
         FROM item_state WHERE item_id = ?1",
        [item_id],
        |row| {
            Ok(ItemState {
                item_id: row.get(0)?,
                stability: row.get(1)?,
                difficulty: row.get(2)?,
                due_at: DateTime::from_timestamp(row.get(3)?, 0)
                    .expect("item_state.due_at is not a valid timestamp"),
                last_reviewed_at: row
                    .get::<_, Option<i64>>(4)?
                    .and_then(|s| DateTime::from_timestamp(s, 0)),
                reps: row.get(5)?,
                lapses: row.get(6)?,
            })
        },
    )?;
    Ok(state)
}

/// Increments the rep counter on a successful recall (Good or Easy).
fn update_reps(rating: Rating, current: u32) -> u32 {
    match rating {
        Rating::Good | Rating::Easy => current + 1,
        _ => current,
    }
}

/// Increments the lapse counter when a learned item is forgotten (Hard or Again).
/// Items still in the learning phase don't accrue lapses.
fn update_lapses(rating: Rating, current: u32, is_learned: bool) -> u32 {
    if is_learned {
        match rating {
            Rating::Hard | Rating::Again => current + 1,
            _ => current,
        }
    } else {
        current
    }
}

/// An item is considered learned once its successful reps outnumber its lapses.
fn is_learned(reps: u32, lapses: u32) -> bool {
    reps > lapses
}
