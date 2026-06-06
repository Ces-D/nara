use crate::error::ServiceResult;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use brainiac_core::database::{
    self,
    connection::BrainiacDbPool,
    models::{
        Category, CategoryTag, CategoryTagLink, CreateCategory, CreateItem, PracticeItem,
        PracticeItemAnswer, PracticeItemsFilters, UpdateItem,
    },
};

pub async fn list_categories(
    State(pool): State<BrainiacDbPool>,
) -> ServiceResult<Json<Vec<(Category, Vec<CategoryTag>)>>> {
    let categories = database::list_categories_with_tags(&pool).await?;
    Ok(Json(categories))
}

pub async fn create_category(
    State(pool): State<BrainiacDbPool>,
    Json(payload): Json<CreateCategory>,
) -> ServiceResult<StatusCode> {
    database::create_category(&pool, payload).await?;
    Ok(StatusCode::CREATED)
}

pub async fn delete_category(
    State(pool): State<BrainiacDbPool>,
    Path(id): Path<i64>,
) -> ServiceResult<StatusCode> {
    let was_deleted = database::delete_category(&pool, id).await?;
    if was_deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

pub async fn add_category_tag(
    State(pool): State<BrainiacDbPool>,
    Json(payload): Json<CategoryTagLink>,
) -> ServiceResult<StatusCode> {
    database::add_tag_to_category(&pool, payload).await?;
    Ok(StatusCode::CREATED)
}

pub async fn list_tags(
    State(pool): State<BrainiacDbPool>,
) -> ServiceResult<Json<Vec<CategoryTag>>> {
    let tags = database::list_tags(&pool).await?;
    Ok(Json(tags))
}

pub async fn remove_category_tag(
    State(pool): State<BrainiacDbPool>,
    Json(payload): Json<CategoryTagLink>,
) -> ServiceResult<StatusCode> {
    let was_removed =
        database::remove_tag_from_category(&pool, payload.category_id, payload.tag_name).await?;
    if was_removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

pub async fn practice(
    State(pool): State<BrainiacDbPool>,
    Json(payload): Json<PracticeItemsFilters>,
) -> ServiceResult<Json<Vec<PracticeItem>>> {
    let items = database::get_practice_items(
        &pool,
        payload.limit,
        payload.category_ids,
        payload.tag_names,
    )
    .await?;
    Ok(Json(items))
}

pub async fn practice_item_answer(
    State(pool): State<BrainiacDbPool>,
    Path(id): Path<i64>,
) -> ServiceResult<Json<PracticeItemAnswer>> {
    let answer = database::get_practice_item_answer(&pool, id).await?;
    Ok(Json(answer))
}

pub async fn create_practice_items(
    State(pool): State<BrainiacDbPool>,
    Json(payload): Json<Vec<CreateItem>>,
) -> ServiceResult<StatusCode> {
    database::create_items(&pool, payload).await?;
    Ok(StatusCode::CREATED)
}

pub async fn edit_practice_item(
    State(pool): State<BrainiacDbPool>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateItem>,
) -> ServiceResult<StatusCode> {
    let was_updated = database::update_item(&pool, id, payload).await?;
    if was_updated {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}
