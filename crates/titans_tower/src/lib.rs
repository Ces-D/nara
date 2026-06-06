pub mod bot;
pub mod channel;
pub mod error;
pub mod provision;
pub mod ui;

pub use bot::{UserFacingError, default_intents, spawn_client};
pub use channel::{
    DISCORD_EMBED_MIME, DiscordChannel, DiscordEmbedField, DiscordEmbedPayload,
    configured_channel_id, configured_channel_ids, register_channels,
};
pub use error::TowerError;
pub use provision::{ensure_text_channel, refresh_pinned_message, require_channel};
pub use ui::{
    COLOR_ACCENT_PRIMARY, COLOR_ACCENT_SECONDARY, FieldParseError, PaginateAction, paginate,
    parse_date, parse_rrule,
};
