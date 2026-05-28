use chrono::{DateTime, NaiveDate, Utc};
use rrule::{RRule, Unvalidated};
use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

// ── Color palette (hex, for embed borders) ──
pub const COLOR_ACCENT_PRIMARY: u32 = 0x14b8a6;
pub const COLOR_ACCENT_SECONDARY: u32 = 0x8b5cf6;

#[derive(Debug, thiserror::Error)]
pub enum FieldParseError {
    #[error("failed to parse `{field}` as date (expected format MM-DD-YYYY, got `{value}`)")]
    InvalidDate { field: String, value: String },
    #[error("failed to parse `{field}` as RRULE (`{value}`)")]
    InvalidRRule { field: String, value: String },
}

pub fn parse_date(field: &str, value: &str) -> Result<DateTime<Utc>, FieldParseError> {
    let naive = NaiveDate::parse_from_str(value, "%m-%d-%Y")
        .map_err(|_| FieldParseError::InvalidDate {
            field: field.to_string(),
            value: value.to_string(),
        })?
        .and_hms_opt(0, 0, 0)
        .unwrap();

    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

/// Parses an RFC 5545 RRULE string and validates it against `start`,
/// returning the unvalidated form so it can be sent to APIs that expect it.
/// Validation runs as a pre-check so invalid rrules surface as user errors here.
pub fn parse_rrule(
    field: &str,
    value: &str,
    start: DateTime<Utc>,
) -> Result<RRule<Unvalidated>, FieldParseError> {
    let unvalidated: RRule<Unvalidated> =
        value.parse().map_err(|_| FieldParseError::InvalidRRule {
            field: field.to_string(),
            value: value.to_string(),
        })?;
    unvalidated
        .clone()
        .validate(start.with_timezone(&rrule::Tz::America__New_York))
        .map_err(|_| FieldParseError::InvalidRRule {
            field: field.to_string(),
            value: value.to_string(),
        })?;
    Ok(unvalidated)
}

/// A terminal choice button shown alongside the prev/next navigation row in
/// [`paginate`]. `id` is a stable key returned to the caller when pressed.
pub struct PaginateAction {
    pub id: &'static str,
    pub label: String,
    pub style: ButtonStyle,
}

/// Sends a paginated embed message with prev/next navigation plus a row of
/// caller-defined action buttons. Returns the `id` of the action the user
/// pressed, or `None` if the interaction timed out. Pressing an action clears
/// the buttons and ends pagination.
pub async fn paginate<Data, Err>(
    ctx: poise::Context<'_, Data, Err>,
    pages: Vec<CreateEmbed>,
    actions: Vec<PaginateAction>,
) -> Result<Option<&'static str>, serenity::Error>
where
    Data: Send + Sync,
    Err: Send + Sync,
{
    let ctx_id = ctx.id();
    let prefix = ctx_id.to_string();
    let prev_id = format!("{prefix}prev");
    let next_id = format!("{prefix}next");

    let action_buttons: Vec<CreateButton> = actions
        .iter()
        .map(|a| {
            CreateButton::new(format!("{prefix}{}", a.id))
                .label(a.label.clone())
                .style(a.style)
        })
        .collect();

    let components = vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new(&prev_id).emoji('◀'),
            CreateButton::new(&next_id).emoji('▶'),
        ]),
        CreateActionRow::Buttons(action_buttons),
    ];

    let reply = poise::CreateReply::default()
        .embed(pages[0].clone())
        .components(components.clone());
    ctx.send(reply).await?;

    let mut current_page = 0usize;
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(std::time::Duration::from_secs(60 * 20))
        .await
    {
        let suffix = press
            .data
            .custom_id
            .strip_prefix(&ctx_id.to_string())
            .unwrap_or("");

        if let Some(action) = actions.iter().find(|a| a.id == suffix) {
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
            return Ok(Some(action.id));
        }

        if suffix == "next" {
            current_page = (current_page + 1) % pages.len();
        } else if suffix == "prev" {
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
