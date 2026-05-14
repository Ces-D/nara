use crate::db::run_brainiac_blocking;
use crate::error::ServiceError;
use brainiac_core::database::{
    self,
    connection::BrainiacDbPool,
    models::{
        Category, CategoryTag, CategoryTagLink, CreateCategory, CreateItem, Item, PracticeItem,
        PracticeItemAnswer, Rating, UpdateItem,
    },
};

pub async fn list_categories(
    pool: BrainiacDbPool,
) -> Result<Vec<(Category, Vec<CategoryTag>)>, ServiceError> {
    run_brainiac_blocking(pool, |conn| Ok(database::list_categories_with_tags(conn)?)).await
}

pub async fn create_category(
    pool: BrainiacDbPool,
    create: CreateCategory,
) -> Result<i64, ServiceError> {
    run_brainiac_blocking(pool, move |conn| {
        Ok(database::create_category(create, conn)?)
    })
    .await
}

pub async fn delete_category(pool: BrainiacDbPool, id: i64) -> Result<bool, ServiceError> {
    run_brainiac_blocking(pool, move |conn| {
        database::delete_category(id, conn)?;
        Ok(conn.changes() > 0)
    })
    .await
}

pub async fn add_category_tag(
    pool: BrainiacDbPool,
    link: CategoryTagLink,
) -> Result<(), ServiceError> {
    run_brainiac_blocking(pool, move |conn| {
        database::add_tag_to_category(link, conn)?;
        Ok(())
    })
    .await
}

pub async fn list_tags(pool: BrainiacDbPool) -> Result<Vec<CategoryTag>, ServiceError> {
    run_brainiac_blocking(pool, |conn| Ok(database::list_tags(conn)?)).await
}

pub async fn remove_category_tag(
    pool: BrainiacDbPool,
    link: CategoryTagLink,
) -> Result<bool, ServiceError> {
    run_brainiac_blocking(pool, move |conn| {
        database::remove_tag_from_category(link.category_id, link.tag_name, conn)?;
        Ok(conn.changes() > 0)
    })
    .await
}

pub async fn fetch_practice_items(
    pool: BrainiacDbPool,
    limit: u8,
    category_ids: Option<Vec<i64>>,
    tag_names: Option<Vec<String>>,
) -> Result<Vec<PracticeItem>, ServiceError> {
    run_brainiac_blocking(pool, move |conn| {
        Ok(database::get_practice_items(
            limit,
            category_ids,
            tag_names,
            conn,
        )?)
    })
    .await
}

pub async fn fetch_practice_and_answer(
    pool: BrainiacDbPool,
    category_ids: Option<Vec<i64>>,
    tag_names: Option<Vec<String>>,
) -> Result<Option<(PracticeItem, PracticeItemAnswer)>, ServiceError> {
    run_brainiac_blocking(pool, move |conn| {
        let Some(item) = database::get_practice_items(1, category_ids, tag_names, conn)?.pop()
        else {
            return Ok(None);
        };
        let answer = database::get_practice_item_answer(item.id, conn)?;
        Ok(Some((item, answer)))
    })
    .await
}

pub async fn get_practice_item_answer(
    pool: BrainiacDbPool,
    id: i64,
) -> Result<PracticeItemAnswer, ServiceError> {
    run_brainiac_blocking(pool, move |conn| {
        Ok(database::get_practice_item_answer(id, conn)?)
    })
    .await
}

pub async fn rate_practice_item(
    pool: BrainiacDbPool,
    id: i64,
    rating: Rating,
) -> Result<(), ServiceError> {
    run_brainiac_blocking(pool, move |conn| {
        database::rate_practice_item(id, rating, conn)?;
        Ok(())
    })
    .await
}

pub async fn create_items(
    pool: BrainiacDbPool,
    items: Vec<CreateItem>,
) -> Result<(), ServiceError> {
    run_brainiac_blocking(pool, move |conn| {
        database::create_items(items, conn)?;
        Ok(())
    })
    .await
}

pub async fn get_item(pool: BrainiacDbPool, id: i64) -> Result<Item, ServiceError> {
    run_brainiac_blocking(pool, move |conn| Ok(database::get_item(id, conn)?)).await
}

pub async fn update_item(
    pool: BrainiacDbPool,
    id: i64,
    update: UpdateItem,
) -> Result<bool, ServiceError> {
    run_brainiac_blocking(pool, move |conn| {
        database::update_item(id, update, conn)?;
        Ok(conn.changes() > 0)
    })
    .await
}
