use brainiac_core::database::models::{PracticeItem, PracticeItemAnswer};
use chrono::{DateTime, NaiveDate, Utc};
use rrule::{RRule, Unvalidated, Validated};
use serenity::all::{CreateEmbed, CreateEmbedFooter};
use std::time::Duration;

pub mod paginate;

// ── Color palette (hex, for embed borders) ──
pub const COLOR_ACCENT_PRIMARY: u32 = 0x14b8a6;
pub const COLOR_ACCENT_SECONDARY: u32 = 0x8b5cf6;
pub const COLOR_HIGHLIGHT: u32 = 0xd1fae5;
pub const COLOR_WARNING: u32 = 0xb91c1c;

const PAGE_SIZE: usize = 1;

pub const FORM_TIMEOUT: Duration = Duration::from_secs(600);
pub const MODAL_TIMEOUT: Duration = Duration::from_secs(300);

pub const EMPTY: &str = "—";
pub const DATE_FMT: &str = "%m-%d-%Y";

#[derive(Debug, thiserror::Error)]
pub enum FieldParseError {
    #[error("failed to parse `{field}` as date (expected format MM-DD-YYYY, got `{value}`)")]
    InvalidDate { field: String, value: String },
    #[error("failed to parse `{field}` as RRULE (`{value}`): {source}")]
    InvalidRRule {
        field: String,
        value: String,
        #[source]
        source: rrule::RRuleError,
    },
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

/// Parses an RFC 5545 RRULE string and validates it against `start`.
pub fn parse_rrule(
    field: &str,
    value: &str,
    start: DateTime<Utc>,
) -> Result<RRule<Validated>, FieldParseError> {
    let unvalidated: RRule<Unvalidated> =
        value
            .parse()
            .map_err(|e: rrule::RRuleError| FieldParseError::InvalidRRule {
                field: field.to_string(),
                value: value.to_string(),
                source: e,
            })?;
    unvalidated
        .validate(start.with_timezone(&rrule::Tz::America__New_York))
        .map_err(|e| FieldParseError::InvalidRRule {
            field: field.to_string(),
            value: value.to_string(),
            source: e,
        })
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
