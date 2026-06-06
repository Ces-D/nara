use crate::discord::{
    AppData, Context,
    ui::{answer_embed, paginate::paginate_with_review, practice_item_embed},
};
use crate::error::ServiceError;
use brainiac_core::database::{
    self,
    models::{CategoryTagLink, CreateCategory, CreateItem, UpdateItem},
};

/// The only channel `/brainiac` commands are allowed to run in.
pub const CHANNEL: &str = "brainiac";

/// Detailed help text pinned to the #brainiac channel on startup.
pub fn help_message() -> String {
    "**Brainiac — flashcard & practice commands**\n\
     Use these in this channel (#brainiac) only.\n\n\
     • `/brainiac list_categories` — list categories with their tags.\n\
     • `/brainiac create_category` — create a category. Args: `name`; option `description`.\n\
     • `/brainiac delete_category` — delete a category by `id`.\n\
     • `/brainiac add_category_tag` — link a tag to a category. Args: `id`, `name`.\n\
     • `/brainiac list_tags` — list all tags.\n\
     • `/brainiac remove_category_tag` — unlink a tag from a category. Args: `id`, `name`.\n\
     • `/brainiac practice` — run a practice session. \
     Options: `category_ids` (comma-separated), `tag_names` (comma-separated).\n\
     • `/brainiac create_item` — create a practice item via modal. Args: `category_id`.\n\
     • `/brainiac edit_item` — edit a practice item via modal. Args: `id`."
        .to_string()
}

/// Parent-command check: restricts `/brainiac` (and subcommands) to #brainiac.
async fn in_brainiac_channel(ctx: Context<'_>) -> Result<bool, ServiceError> {
    titans_tower::require_channel(ctx, CHANNEL).await
}

#[poise::command(slash_command)]
pub async fn list_categories(ctx: Context<'_>) -> Result<(), ServiceError> {
    ctx.defer().await?;
    let categories = database::list_categories_with_tags(&ctx.data().brainiac_pool).await?;
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
) -> Result<(), ServiceError> {
    ctx.defer().await?;
    let create = CreateCategory {
        name: name.clone(),
        description,
        created_at: None,
    };
    let id = database::create_category(&ctx.data().brainiac_pool, create).await?;
    ctx.say(format!("Category '{}' created (id: {id}).", name))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn delete_category(
    ctx: Context<'_>,
    #[description = "Id of the Category"] id: i64,
) -> Result<(), ServiceError> {
    ctx.defer().await?;
    let was_deleted = database::delete_category(&ctx.data().brainiac_pool, id).await?;
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
) -> Result<(), ServiceError> {
    ctx.defer().await?;
    let link = CategoryTagLink {
        category_id: id,
        tag_name: name.clone(),
    };
    database::add_tag_to_category(&ctx.data().brainiac_pool, link).await?;
    ctx.say(format!(
        "Tag '{}' linked to category {id}.",
        name.trim().to_uppercase()
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn list_tags(ctx: Context<'_>) -> Result<(), ServiceError> {
    ctx.defer().await?;
    let tags = database::list_tags(&ctx.data().brainiac_pool).await?;
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
) -> Result<(), ServiceError> {
    ctx.defer().await?;
    let was_removed =
        database::remove_tag_from_category(&ctx.data().brainiac_pool, id, name.clone()).await?;
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

#[poise::command(slash_command)]
pub async fn practice(
    ctx: Context<'_>,
    #[description = "Comma separated category ids"] category_ids: Option<String>,
    #[description = "Comma separated tag names"] tag_names: Option<String>,
) -> Result<(), ServiceError> {
    ctx.defer().await?;
    let category_ids: Option<Vec<i64>> = category_ids.map(|v| {
        v.split(",")
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect()
    });
    let tag_names: Option<Vec<String>> =
        tag_names.map(|v| v.split(",").map(|s| s.trim().to_string()).collect());
    loop {
        match database::get_practice_item_with_answer(
            &ctx.data().brainiac_pool,
            category_ids.clone(),
            tag_names.clone(),
        )
        .await?
        {
            Some((practice, answer)) => {
                let pages = vec![practice_item_embed(&practice), answer_embed(&answer)];
                match paginate_with_review(ctx, pages).await? {
                    Some(rating) => {
                        database::rate_practice_item(
                            &ctx.data().brainiac_pool,
                            practice.id,
                            rating,
                        )
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
    ctx: poise::ApplicationContext<'_, AppData, ServiceError>,
    #[description = "Id of the Category"] category_id: i64,
) -> Result<(), ServiceError> {
    use poise::Modal as _;
    let Some(data) = PracticeItemModal::execute(ctx).await? else {
        return Ok(());
    };
    let item = CreateItem {
        category_id,
        front: data.question,
        back: data.answer,
        created_at: None,
    };
    database::create_items(&ctx.data().brainiac_pool, vec![item]).await?;
    poise::Context::Application(ctx)
        .say(format!("Practice item created in category {category_id}."))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn edit_item(
    ctx: poise::ApplicationContext<'_, AppData, ServiceError>,
    #[description = "Id of the Item"] id: i64,
) -> Result<(), ServiceError> {
    let existing = database::get_item(&ctx.data().brainiac_pool, id).await?;
    let prefilled = PracticeItemModal {
        question: existing.front,
        answer: existing.back,
    };
    let Some(data) = poise::execute_modal(ctx, Some(prefilled), None).await? else {
        return Ok(());
    };
    let update = UpdateItem {
        front: Some(data.question),
        back: Some(data.answer),
    };
    database::update_item(&ctx.data().brainiac_pool, id, update).await?;
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
    subcommand_required,
    check = "in_brainiac_channel"
)]
pub async fn brainiac(_ctx: Context<'_>) -> Result<(), ServiceError> {
    Ok(())
}
