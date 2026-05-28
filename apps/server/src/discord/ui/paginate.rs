use crate::discord::Context;
use crate::error::ServiceError;
use brainiac_core::database::models::Rating;
use serenity::all::{ButtonStyle, CreateEmbed};
use titans_tower::{PaginateAction, paginate};

pub async fn paginate_with_review(
    ctx: Context<'_>,
    pages: Vec<CreateEmbed>,
) -> Result<Option<Rating>, ServiceError> {
    let actions = vec![
        PaginateAction {
            id: "again",
            label: "Again".into(),
            style: ButtonStyle::Danger,
        },
        PaginateAction {
            id: "hard",
            label: "Hard".into(),
            style: ButtonStyle::Secondary,
        },
        PaginateAction {
            id: "good",
            label: "Good".into(),
            style: ButtonStyle::Primary,
        },
        PaginateAction {
            id: "easy",
            label: "Easy".into(),
            style: ButtonStyle::Success,
        },
    ];

    let rating = paginate(ctx, pages, actions).await?.map(|id| match id {
        "again" => Rating::Again,
        "hard" => Rating::Hard,
        "good" => Rating::Good,
        "easy" => Rating::Easy,
        other => unreachable!("unexpected paginate action id: {other}"),
    });
    Ok(rating)
}
