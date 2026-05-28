use crate::error::TowerError;
use async_trait::async_trait;
use cadence_core::{
    channels::{Artifact, ChannelRegistry, DeliveryChannel},
    error::CadenceError,
};
use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, CreateEmbed, CreateMessage, Http};
use std::sync::Arc;

pub const DISCORD_EMBED_MIME: &str = "application/x-discord-embed+json";

const DISCORD_CHANNELS_ENV: &str = "NARA_DISCORD_CHANNELS";
const DISCORD_MAX_MESSAGE_LEN: usize = 2000;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscordEmbedPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<DiscordEmbedField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbedField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

impl DiscordEmbedPayload {
    fn into_create_embed(self) -> CreateEmbed {
        let mut embed = CreateEmbed::new();
        if let Some(t) = self.title {
            embed = embed.title(t);
        }
        if let Some(d) = self.description {
            embed = embed.description(d);
        }
        if let Some(c) = self.color {
            embed = embed.colour(c);
        }
        for f in self.fields {
            embed = embed.field(f.name, f.value, f.inline);
        }
        embed
    }
}

pub struct DiscordChannel {
    name: &'static str,
    channel_id: ChannelId,
    http: Arc<Http>,
}

#[async_trait]
impl DeliveryChannel for DiscordChannel {
    fn name(&self) -> &'static str {
        self.name
    }

    fn accepts(&self, artifact: &Artifact) -> bool {
        match artifact {
            Artifact::PlainText(_) => true,
            Artifact::Bytes { mime, .. } => mime == DISCORD_EMBED_MIME,
            Artifact::MarkdownFile(_) => false,
        }
    }

    async fn deliver(&self, artifact: Artifact) -> Result<(), CadenceError> {
        match artifact {
            Artifact::PlainText(text) => {
                for chunk in chunk_message(&text, DISCORD_MAX_MESSAGE_LEN) {
                    self.channel_id
                        .say(&self.http, chunk)
                        .await
                        .map_err(|e| CadenceError::Channel(e.to_string()))?;
                }
                Ok(())
            }
            Artifact::Bytes { mime, data } if mime == DISCORD_EMBED_MIME => {
                let payload: DiscordEmbedPayload = serde_json::from_slice(&data)
                    .map_err(|e| CadenceError::Channel(e.to_string()))?;
                let embed = payload.into_create_embed();
                self.channel_id
                    .send_message(&self.http, CreateMessage::new().embed(embed))
                    .await
                    .map_err(|e| CadenceError::Channel(e.to_string()))?;
                Ok(())
            }
            _ => Err(CadenceError::ArtifactNotAccepted),
        }
    }
}

fn chunk_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut current = String::with_capacity(max_len);
    for ch in text.chars() {
        if current.len() + ch.len_utf8() > max_len {
            chunks.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// Registers a [`DiscordChannel`] for every entry declared in the
/// `NARA_DISCORD_CHANNELS` env var. Format: `name:channel_id,name:channel_id,...`
/// — each entry becomes a channel named `discord.<name>`. Missing env means no
/// Discord channels are registered (producers addressing them will get
/// `CadenceError::NoChannel`).
pub fn register_channels(
    registry: &mut ChannelRegistry,
    http: Arc<Http>,
) -> Result<(), TowerError> {
    let Ok(spec) = std::env::var(DISCORD_CHANNELS_ENV) else {
        return Ok(());
    };
    for entry in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (raw_name, id_str) = entry.split_once(':').ok_or_else(|| {
            TowerError::ChannelConfig(format!(
                "invalid {DISCORD_CHANNELS_ENV} entry `{entry}` — expected `name:channel_id`"
            ))
        })?;
        let id: u64 = id_str.trim().parse().map_err(|e| {
            TowerError::ChannelConfig(format!(
                "invalid channel id in {DISCORD_CHANNELS_ENV} entry `{entry}`: {e}"
            ))
        })?;
        let name: &'static str = Box::leak(format!("discord.{}", raw_name.trim()).into_boxed_str());
        registry.register(DiscordChannel {
            name,
            channel_id: ChannelId::new(id),
            http: http.clone(),
        });
        log::info!("Registered discord channel `{name}` -> {id}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_channel() -> DiscordChannel {
        DiscordChannel {
            name: "discord.test",
            channel_id: ChannelId::new(1),
            http: Arc::new(Http::new("test-token")),
        }
    }

    #[test]
    fn accepts_plain_text_and_embed_bytes() {
        let ch = fake_channel();
        assert!(ch.accepts(&Artifact::PlainText("hi".into())));
        assert!(ch.accepts(&Artifact::Bytes {
            data: b"{}".to_vec(),
            mime: DISCORD_EMBED_MIME.into(),
        }));
        assert!(!ch.accepts(&Artifact::Bytes {
            data: vec![],
            mime: "application/octet-stream".into(),
        }));
        assert!(!ch.accepts(&Artifact::MarkdownFile("/tmp/x.md".into())));
    }

    #[test]
    fn embed_payload_roundtrips_through_serde() {
        let payload = DiscordEmbedPayload {
            title: Some("hello".into()),
            description: Some("world".into()),
            color: Some(0x00ff00),
            fields: vec![DiscordEmbedField {
                name: "key".into(),
                value: "val".into(),
                inline: true,
            }],
        };
        let bytes = serde_json::to_vec(&payload).unwrap();
        let back: DiscordEmbedPayload = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.title.as_deref(), Some("hello"));
        assert_eq!(back.color, Some(0x00ff00));
        assert_eq!(back.fields.len(), 1);
        assert_eq!(back.fields[0].name, "key");
    }

    #[test]
    fn chunk_message_splits_on_length_boundary() {
        let s = "a".repeat(2500);
        let chunks = chunk_message(&s, 2000);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 2000);
        assert_eq!(chunks[1].len(), 500);
    }

    #[test]
    fn chunk_message_short_input_is_single_chunk() {
        let chunks = chunk_message("hello", 2000);
        assert_eq!(chunks, vec!["hello".to_string()]);
    }
}
