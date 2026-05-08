use crate::{AppError, Context};
use brainiac_core::database::models::Rating;
use serenity::all::{
    ButtonStyle, ComponentInteraction, CreateActionRow, CreateButton, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

pub async fn paginate_with_review(
    ctx: Context<'_>,
    pages: Vec<CreateEmbed>,
) -> Result<Option<Rating>, AppError> {
    let ctx_id = ctx.id();
    let prev_id = format!("{}prev", ctx_id);
    let next_id = format!("{}next", ctx_id);
    let again_id = format!("{}again", ctx_id);
    let hard_id = format!("{}hard", ctx_id);
    let good_id = format!("{}good", ctx_id);
    let easy_id = format!("{}easy", ctx_id);

    let components = vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new(&prev_id).emoji('◀'),
            CreateButton::new(&next_id).emoji('▶'),
        ]),
        CreateActionRow::Buttons(vec![
            CreateButton::new(&again_id)
                .label("Again")
                .style(ButtonStyle::Danger),
            CreateButton::new(&hard_id)
                .label("Hard")
                .style(ButtonStyle::Secondary),
            CreateButton::new(&good_id)
                .label("Good")
                .style(ButtonStyle::Primary),
            CreateButton::new(&easy_id)
                .label("Easy")
                .style(ButtonStyle::Success),
        ]),
    ];

    let reply = poise::CreateReply::default()
        .embed(pages[0].clone())
        .components(components.clone());
    ctx.send(reply).await?;

    let mut current_page = 0;
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(std::time::Duration::from_secs(60 * 20))
        .await
    {
        let custom_id = press.data.custom_id.clone();
        let rating = if custom_id == again_id {
            Some(Rating::Again)
        } else if custom_id == hard_id {
            Some(Rating::Hard)
        } else if custom_id == good_id {
            Some(Rating::Good)
        } else if custom_id == easy_id {
            Some(Rating::Easy)
        } else {
            None
        };

        if let Some(rating) = rating {
            handle_review_press(ctx, &press, &pages, current_page).await?;
            return Ok(Some(rating));
        }

        if custom_id == next_id {
            current_page = (current_page + 1) % pages.len();
        } else if custom_id == prev_id {
            current_page = current_page.checked_sub(1).unwrap_or(pages.len() - 1);
        } else {
            continue;
        }

        press
            .create_response(
                ctx.serenity_context(),
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(pages[current_page].clone())
                        .components(components.clone()),
                ),
            )
            .await?;
    }

    Ok(None)
}

async fn handle_review_press(
    ctx: Context<'_>,
    press: &ComponentInteraction,
    pages: &[CreateEmbed],
    current_page: usize,
) -> Result<(), AppError> {
    press
        .create_response(
            ctx.serenity_context(),
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(pages[current_page].clone())
                    .components(vec![]),
            ),
        )
        .await?;
    Ok(())
}
