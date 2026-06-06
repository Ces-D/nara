//! Reading a day's worth of `#daily-dilly` messages from Discord, grouped by
//! author. This is the feature's only Discord *read* concern.

use cadence_core::error::CadenceError;
use serenity::all::{ChannelId, GetMessages, Http, MessageId};
use std::collections::HashMap;

/// Safety cap on how many messages we pull from the channel in one run.
const MAX_MESSAGES_PER_CHANNEL: usize = 5000;

/// One person's contributions for the day.
pub struct PersonDay {
    /// Unique Discord username — used as the bean category name.
    pub username: String,
    /// Friendly display name for the posted summary.
    pub display: String,
    /// Message contents, oldest first.
    pub messages: Vec<String>,
}

/// Page through the channel (newest first) collecting non-bot messages from
/// on/after `start_ts`, grouped by author username (which is globally unique, so
/// it makes a stable per-person category key).
pub async fn collect_day_messages(
    http: &Http,
    channel_id: ChannelId,
    start_ts: i64,
) -> Result<HashMap<String, PersonDay>, CadenceError> {
    let mut by_author: HashMap<String, PersonDay> = HashMap::new();
    let mut before: Option<MessageId> = None;
    let mut total = 0usize;

    'outer: loop {
        let mut builder = GetMessages::new().limit(100);
        if let Some(b) = before {
            builder = builder.before(b);
        }
        let batch = channel_id
            .messages(http, builder)
            .await
            .map_err(|e| CadenceError::Channel(e.to_string()))?;
        if batch.is_empty() {
            break;
        }
        let oldest_id = batch.last().map(|m| m.id);

        for message in batch {
            // Newest-first ordering: once we cross the day boundary, every
            // remaining message is older too.
            if message.timestamp.unix_timestamp() < start_ts {
                break 'outer;
            }
            if message.author.bot || message.content.trim().is_empty() {
                continue;
            }
            let username = message.author.name.clone();
            let display = message
                .author
                .global_name
                .clone()
                .unwrap_or_else(|| message.author.name.clone());
            let entry = by_author
                .entry(username.clone())
                .or_insert_with(|| PersonDay {
                    username,
                    display,
                    messages: Vec::new(),
                });
            entry.messages.push(message.content.clone());
            total += 1;
        }

        if total >= MAX_MESSAGES_PER_CHANNEL {
            log::warn!("daily-dilly: hit message cap, summarizing a partial day");
            break;
        }
        match oldest_id {
            Some(id) => before = Some(id),
            None => break,
        }
    }

    // Messages were collected newest-first; flip to chronological order.
    for person in by_author.values_mut() {
        person.messages.reverse();
    }
    Ok(by_author)
}
