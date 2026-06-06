use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Rating ──

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum Rating {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

// ── Category ──

#[derive(Debug, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_practiced: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCategory {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, with = "chrono::serde::ts_seconds_option")]
    pub created_at: Option<DateTime<Utc>>,
}

// ── Category Tag ──

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryTag {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryTagLink {
    pub category_id: i64,
    pub tag_name: String,
}

// ── Item ──

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub category_id: i64,
    pub front: String,
    pub back: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PracticeItem {
    pub id: i64,
    pub front: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PracticeItemAnswer {
    pub id: i64,
    pub back: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateItem {
    pub front: Option<String>,
    pub back: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateItem {
    pub category_id: i64,
    pub front: String,
    pub back: String,
    #[serde(default, with = "chrono::serde::ts_seconds_option")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PracticeItemsFilters {
    pub limit: u8,
    pub category_ids: Option<Vec<i64>>,
    pub tag_names: Option<Vec<String>>,
}

// ── Item State (FSRS scheduling) ──

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemState {
    pub item_id: i64,
    pub stability: Option<f32>,
    pub difficulty: Option<f32>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub due_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_reviewed_at: Option<DateTime<Utc>>,
    pub reps: u32,
    pub lapses: u32,
}
