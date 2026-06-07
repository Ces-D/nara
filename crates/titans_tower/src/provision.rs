use serenity::all::{CreateMessage, GuildChannel, Http, UserId};

/// (Re)pins `content` as the bot's help message in `channel`. Any pins
/// previously authored by `bot_id` are removed first so the message can be
/// refreshed without piling up duplicates. Requires the Manage Messages and
/// Read Message History permissions.
pub async fn refresh_pinned_message(
    http: &Http,
    channel: &GuildChannel,
    bot_id: UserId,
    content: &str,
) -> Result<(), serenity::Error> {
    for pin in channel.pins(http).await? {
        if pin.author.id == bot_id {
            pin.unpin(http).await?;
        }
    }
    let message = channel
        .send_message(http, CreateMessage::new().content(content))
        .await?;
    message.pin(http).await?;
    Ok(())
}

/// A poise command check that passes only when the command was invoked in the
/// Discord channel configured for `key` in `NARA_DISCORD_CHANNELS` (matched by
/// channel id — the channel's display name is irrelevant). On mismatch, or when
/// `key` is not configured, it replies ephemerally and fails the check (so the
/// command body never runs). `key` is the service's `CHANNEL` const. Wrap it in
/// a zero-argument check function per command, since poise `check` attributes
/// can't take arguments.
pub async fn require_channel<D, E>(ctx: poise::Context<'_, D, E>, key: &str) -> Result<bool, E>
where
    D: Send + Sync,
    E: From<serenity::Error> + Send + Sync,
{
    let Ok(configured) = crate::channel::configured_channel_id(key) else {
        ctx.send(
            poise::CreateReply::default()
                .content("This command's channel is not configured.")
                .ephemeral(true),
        )
        .await?;
        return Ok(false);
    };

    let in_channel = ctx.channel_id() == configured;
    if !in_channel {
        ctx.send(
            poise::CreateReply::default()
                .content(format!("This command can only be used in <#{configured}>."))
                .ephemeral(true),
        )
        .await?;
    }
    Ok(in_channel)
}
