use serenity::all::{
    ChannelType, CreateChannel, CreateMessage, GuildChannel, GuildId, Http, UserId,
};

/// Ensures a text channel named `name` exists in `guild`, returning it. If no
/// matching text channel is found it is created (requires the bot to hold the
/// Manage Channels permission in the guild).
pub async fn ensure_text_channel(
    http: &Http,
    guild: GuildId,
    name: &str,
) -> Result<GuildChannel, serenity::Error> {
    let existing = guild
        .channels(http)
        .await?
        .into_values()
        .find(|c| c.kind == ChannelType::Text && c.name == name);
    match existing {
        Some(channel) => Ok(channel),
        None => {
            guild
                .create_channel(http, CreateChannel::new(name).kind(ChannelType::Text))
                .await
        }
    }
}

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

/// A poise command check that passes only when the command was invoked in a
/// guild channel named `name`. Otherwise it replies with a hint and fails the
/// check (so the command body never runs). Wrap it in a zero-argument check
/// function per command, since poise `check` attributes can't take arguments.
pub async fn require_channel<D, E>(ctx: poise::Context<'_, D, E>, name: &str) -> Result<bool, E>
where
    D: Send + Sync,
    E: From<serenity::Error> + Send + Sync,
{
    let in_channel = ctx
        .guild_channel()
        .await
        .map(|c| c.name == name)
        .unwrap_or(false);
    if !in_channel {
        ctx.say(format!("This command can only be used in #{name}."))
            .await?;
    }
    Ok(in_channel)
}
