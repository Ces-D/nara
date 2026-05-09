use crate::{
    AppData, AppError, Context,
    ui::{answer_embed, paginate::paginate_with_review, practice_item_embed},
};
use brainiac_core::database::{
    self, BrainiacDbError,
    connection::BrainiacDbPool,
    models::{CreateCategory, CreateItem, PracticeItem, PracticeItemAnswer, UpdateItem},
};

async fn run_db_blocking<F, T>(pool: BrainiacDbPool, f: F) -> Result<T, AppError>
where
    F: FnOnce(
            &mut brainiac_core::database::connection::BrainiacDbPoolConnection,
        ) -> Result<T, AppError>
        + Send
        + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || -> Result<T, AppError> {
        let mut conn = pool.get().map_err(BrainiacDbError::from)?;
        f(&mut conn)
    })
    .await?
}

#[poise::command(slash_command)]
pub async fn list_categories(ctx: Context<'_>) -> Result<(), AppError> {
    ctx.defer().await?;
    let pool = ctx.data().brainiac_pool.clone();
    let categories =
        run_db_blocking(pool, |conn| Ok(database::list_categories_with_tags(conn)?)).await?;
    if categories.is_empty() {
        ctx.say("No categories created").await?;
        return Ok(());
    }
    for (category, tags) in categories {
        ctx.say(format!(
            "Category '{}' (id: {})",
            category.name, category.id
        ))
        .await?;
        if let Some(category_description) = category.description {
            ctx.say(format!("Description: {}", category_description))
                .await?;
        }
        if !tags.is_empty() {
            ctx.say(format!(
                "Tags: {}",
                tags.iter()
                    .map(|v| v.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command)]
pub async fn create_category(
    ctx: Context<'_>,
    #[description = "Name of the Category"] name: String,
    #[description = "Description of the Category"] description: Option<String>,
) -> Result<(), AppError> {
    ctx.defer().await?;
    let pool = ctx.data().brainiac_pool.clone();
    let create = CreateCategory {
        name: name.clone(),
        description,
        created_at: None,
    };
    let id = run_db_blocking(pool, move |conn| {
        Ok(database::create_category(create, conn)?)
    })
    .await?;
    ctx.say(format!("Category '{}' created (id: {id}).", name))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn delete_category(
    ctx: Context<'_>,
    #[description = "Id of the Category"] id: i64,
) -> Result<(), AppError> {
    ctx.defer().await?;
    let pool = ctx.data().brainiac_pool.clone();
    let was_deleted = run_db_blocking(pool, move |conn| {
        database::delete_category(id, conn)?;
        Ok(conn.changes() > 0)
    })
    .await?;
    let msg = if !was_deleted {
        format!("No category found with id {id}.")
    } else {
        format!("Category {id} deleted.")
    };
    ctx.say(msg).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn add_category_tag(
    ctx: Context<'_>,
    #[description = "Id of the Category"] id: i64,
    #[description = "Name of the Tag"] name: String,
) -> Result<(), AppError> {
    ctx.defer().await?;
    let pool = ctx.data().brainiac_pool.clone();
    let name_for_db = name.clone();
    run_db_blocking(pool, move |conn| {
        database::add_tag_to_category(id, name_for_db, conn)?;
        Ok(())
    })
    .await?;
    ctx.say(format!(
        "Tag '{}' linked to category {id}.",
        name.trim().to_uppercase()
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn list_tags(ctx: Context<'_>) -> Result<(), AppError> {
    ctx.defer().await?;
    let pool = ctx.data().brainiac_pool.clone();
    let tags = run_db_blocking(pool, |conn| Ok(database::list_tags(conn)?)).await?;
    if tags.is_empty() {
        ctx.say("No tags created").await?;
    } else {
        ctx.say(format!(
            "Tags: {}",
            tags.iter()
                .map(|v| v.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .await?;
    }
    Ok(())
}

#[poise::command(slash_command)]
pub async fn remove_category_tag(
    ctx: Context<'_>,
    #[description = "Id of the Category"] id: i64,
    #[description = "Name of the Tag"] name: String,
) -> Result<(), AppError> {
    ctx.defer().await?;
    let pool = ctx.data().brainiac_pool.clone();
    let name_for_db = name.clone();
    let was_removed = run_db_blocking(pool, move |conn| {
        database::remove_tag_from_category(id, name_for_db, conn)?;
        Ok(conn.changes() > 0)
    })
    .await?;
    let msg = if !was_removed {
        format!(
            "Tag '{}' was not linked to category {id}.",
            name.trim().to_uppercase()
        )
    } else {
        format!(
            "Tag '{}' removed from category {id}.",
            name.trim().to_uppercase()
        )
    };
    ctx.say(msg).await?;
    Ok(())
}

async fn fetch_practice_and_answer(
    pool: BrainiacDbPool,
    category_ids: Option<Vec<i64>>,
    tag_names: Option<Vec<String>>,
) -> Result<Option<(PracticeItem, PracticeItemAnswer)>, AppError> {
    run_db_blocking(pool, move |conn| {
        let Some(item) = database::get_practice_items(1, category_ids, tag_names, conn)?.pop()
        else {
            return Ok(None);
        };
        let answer = database::get_practice_item_answer(item.id, conn)?;
        Ok(Some((item, answer)))
    })
    .await
}

#[poise::command(slash_command)]
pub async fn practice(
    ctx: Context<'_>,
    #[description = "Comma separated category ids"] category_ids: Option<String>,
    #[description = "Comma separated tag names"] tag_names: Option<String>,
) -> Result<(), AppError> {
    ctx.defer().await?;
    let category_ids: Option<Vec<i64>> = category_ids.map(|v| {
        v.split(",")
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect()
    });
    let tag_names: Option<Vec<String>> =
        tag_names.map(|v| v.split(",").map(|s| s.trim().to_string()).collect());
    loop {
        let pool = ctx.data().brainiac_pool.clone();
        match fetch_practice_and_answer(pool, category_ids.clone(), tag_names.clone()).await? {
            Some((practice, answer)) => {
                let pages = vec![practice_item_embed(&practice), answer_embed(&answer)];
                match paginate_with_review(ctx, pages).await? {
                    Some(rating) => {
                        let pool = ctx.data().brainiac_pool.clone();
                        let practice_id = practice.id;
                        run_db_blocking(pool, move |conn| {
                            database::rate_practice_item(practice_id, rating, conn)?;
                            Ok(())
                        })
                        .await?;
                    }
                    None => break,
                }
            }
            None => {
                ctx.say("No more practice items for these filters.").await?;
                break;
            }
        }
    }
    Ok(())
}

#[derive(Debug, poise::Modal)]
#[name = "Practice Item"]
struct PracticeItemModal {
    #[name = "Question"]
    #[paragraph]
    #[max_length = 4000]
    question: String,
    #[name = "Answer"]
    #[paragraph]
    #[max_length = 4000]
    answer: String,
}

#[poise::command(slash_command)]
pub async fn create_item(
    ctx: poise::ApplicationContext<'_, AppData, AppError>,
    #[description = "Id of the Category"] category_id: i64,
) -> Result<(), AppError> {
    use poise::Modal as _;
    let Some(data) = PracticeItemModal::execute(ctx).await? else {
        return Ok(());
    };
    let pool = ctx.data().brainiac_pool.clone();
    let item = CreateItem {
        category_id,
        front: data.question,
        back: data.answer,
        created_at: None,
    };
    run_db_blocking(pool, move |conn| {
        database::create_items(vec![item], conn)?;
        Ok(())
    })
    .await?;
    poise::Context::Application(ctx)
        .say(format!("Practice item created in category {category_id}."))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn edit_item(
    ctx: poise::ApplicationContext<'_, AppData, AppError>,
    #[description = "Id of the Item"] id: i64,
) -> Result<(), AppError> {
    let pool = ctx.data().brainiac_pool.clone();
    let existing = run_db_blocking(pool, move |conn| Ok(database::get_item(id, conn)?)).await?;
    let prefilled = PracticeItemModal {
        question: existing.front,
        answer: existing.back,
    };
    let Some(data) = poise::execute_modal(ctx, Some(prefilled), None).await? else {
        return Ok(());
    };
    let pool = ctx.data().brainiac_pool.clone();
    let update = UpdateItem {
        front: Some(data.question),
        back: Some(data.answer),
    };
    run_db_blocking(pool, move |conn| {
        database::update_item(id, update, conn)?;
        Ok(())
    })
    .await?;
    poise::Context::Application(ctx)
        .say(format!("Practice item {id} updated."))
        .await?;
    Ok(())
}

#[poise::command(
    slash_command,
    subcommands(
        "list_categories",
        "create_category",
        "delete_category",
        "add_category_tag",
        "list_tags",
        "remove_category_tag",
        "practice",
        "create_item",
        "edit_item",
    ),
    subcommand_required
)]
pub async fn brainiac(_ctx: Context<'_>) -> Result<(), AppError> {
    Ok(())
}
