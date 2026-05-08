use crate::{
    AppData, AppError, Context,
    ui::{answer_embed, paginate::paginate_with_review, practice_item_embed},
};
use brainiac_core::database::{
    self, BrainiacDbError,
    connection::BrainiacDbPoolConnection,
    models::{CreateCategory, CreateItem, PracticeItem, PracticeItemAnswer, UpdateItem},
};

#[poise::command(slash_command)]
pub async fn list_categories(ctx: Context<'_>) -> Result<(), AppError> {
    let conn = ctx
        .data()
        .brainiac_pool
        .get()
        .map_err(BrainiacDbError::from)?;
    let categories = database::list_categories_with_tags(&conn)?;
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
    let conn = ctx
        .data()
        .brainiac_pool
        .get()
        .map_err(BrainiacDbError::from)?;
    let id = database::create_category(
        CreateCategory {
            name: name.clone(),
            description,
            created_at: None,
        },
        &conn,
    )?;
    ctx.say(format!("Category '{}' created (id: {id}).", name))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn delete_category(
    ctx: Context<'_>,
    #[description = "Id of the Category"] id: i64,
) -> Result<(), AppError> {
    let conn = ctx
        .data()
        .brainiac_pool
        .get()
        .map_err(BrainiacDbError::from)?;
    database::delete_category(id, &conn)?;
    let msg = if conn.changes() == 0 {
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
    let mut conn = ctx
        .data()
        .brainiac_pool
        .get()
        .map_err(BrainiacDbError::from)?;
    database::add_tag_to_category(id, name.clone(), &mut conn)?;
    ctx.say(format!(
        "Tag '{}' linked to category {id}.",
        name.trim().to_uppercase()
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn list_tags(ctx: Context<'_>) -> Result<(), AppError> {
    let conn = ctx
        .data()
        .brainiac_pool
        .get()
        .map_err(BrainiacDbError::from)?;
    let tags = database::list_tags(&conn)?;
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
    let conn = ctx
        .data()
        .brainiac_pool
        .get()
        .map_err(BrainiacDbError::from)?;
    database::remove_tag_from_category(id, name.clone(), &conn)?;
    let msg = if conn.changes() == 0 {
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
    conn: BrainiacDbPoolConnection,
    category_ids: Option<Vec<i64>>,
    tag_names: Option<Vec<String>>,
) -> Result<Option<(PracticeItem, PracticeItemAnswer)>, AppError> {
    tokio::task::spawn_blocking(move || {
        let Some(item) = database::get_practice_items(1, category_ids, tag_names, &conn)?.pop()
        else {
            return Ok(None);
        };
        let answer = database::get_practice_item_answer(item.id, &conn)?;
        Ok(Some((item, answer)))
    })
    .await?
}

#[poise::command(slash_command)]
pub async fn practice(
    ctx: Context<'_>,
    #[description = "Comma separated category ids"] category_ids: Option<String>,
    #[description = "Comma separated tag names"] tag_names: Option<String>,
) -> Result<(), AppError> {
    let category_ids: Option<Vec<i64>> = category_ids.map(|v| {
        v.split(",")
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect()
    });
    let tag_names: Option<Vec<String>> =
        tag_names.map(|v| v.split(",").map(|s| s.trim().to_string()).collect());
    loop {
        let conn = ctx
            .data()
            .brainiac_pool
            .get()
            .map_err(BrainiacDbError::from)?;
        match fetch_practice_and_answer(conn, category_ids.clone(), tag_names.clone()).await? {
            Some((practice, answer)) => {
                let pages = vec![practice_item_embed(&practice), answer_embed(&answer)];
                match paginate_with_review(ctx, pages).await? {
                    Some(rating) => {
                        let conn2 = ctx
                            .data()
                            .brainiac_pool
                            .get()
                            .map_err(BrainiacDbError::from)?;
                        database::rate_practice_item(practice.id, rating, &conn2)?;
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
    let mut conn = ctx
        .data()
        .brainiac_pool
        .get()
        .map_err(BrainiacDbError::from)?;
    database::create_items(
        vec![CreateItem {
            category_id,
            front: data.question,
            back: data.answer,
            created_at: None,
        }],
        &mut conn,
    )?;
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
    let conn = ctx
        .data()
        .brainiac_pool
        .get()
        .map_err(BrainiacDbError::from)?;
    let existing = database::get_item(id, &conn)?;
    let prefilled = PracticeItemModal {
        question: existing.front,
        answer: existing.back,
    };
    let Some(data) = poise::execute_modal(ctx, Some(prefilled), None).await? else {
        return Ok(());
    };
    database::update_item(
        id,
        UpdateItem {
            front: Some(data.question),
            back: Some(data.answer),
        },
        &conn,
    )?;
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
