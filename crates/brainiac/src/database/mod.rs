use std::path::PathBuf;

pub mod connection;
pub mod models;
mod operations;

pub use operations::{
    add_tag_to_category, create_category, create_items, delete_category, delete_item, get_item,
    get_practice_item_answer, get_practice_items, list_categories_with_tags, list_tags,
    rate_practice_item, remove_tag_from_category, update_item,
};

fn application_storage() -> PathBuf {
    let home = std::env::home_dir().expect("Unable to find HOME env variable");
    let p = home.join(".local/share/brainiac");
    if !p.exists() {
        std::fs::create_dir_all(&p).expect(&format!(
            "Unable to create brainiac storage directory at: {}",
            p.display()
        ));
    }
    p
}

fn brainiac_database() -> PathBuf {
    application_storage().join("brainiac.db")
}

#[derive(Debug, thiserror::Error)]
pub enum BrainiacDbError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
    #[error(transparent)]
    Fsrs(#[from] fsrs::FSRSError),
}
