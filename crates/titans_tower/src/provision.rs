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
        ctx.send(
            poise::CreateReply::default()
                .content(format!("This command can only be used in #{name}."))
                .ephemeral(true),
        )
        .await?;
    }
    Ok(in_channel)
}
