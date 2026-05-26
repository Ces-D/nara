use brainiac_core::database::models::{PracticeItem, PracticeItemAnswer};
use chrono::{DateTime, NaiveDate, Utc};
use rrule::{RRule, Unvalidated};
use serenity::all::{CreateEmbed, CreateEmbedFooter};

pub mod paginate;

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

pub fn practice_item_embed(item: &PracticeItem) -> CreateEmbed {
    CreateEmbed::new()
        .title("Question")
        .description(format!("## {}", item.front))
        .color(COLOR_ACCENT_PRIMARY)
        .footer(CreateEmbedFooter::new(format!("ID: {}", item.id)))
}

pub fn answer_embed(item: &PracticeItemAnswer) -> CreateEmbed {
    CreateEmbed::new()
        .title("Answer")
        .description(format!("## {}", item.back))
        .color(COLOR_ACCENT_SECONDARY)
        .footer(CreateEmbedFooter::new(format!("ID: {}", item.id)))
}
