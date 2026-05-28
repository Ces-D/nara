use serenity::all::{Client, GatewayIntents};

/// Lets the generic [`spawn_client`] error handler decide whether a command
/// error is safe to echo back to the user or should be replaced with a generic
/// message and logged. Implement this for your framework error type.
pub trait UserFacingError {
    fn is_user_facing(&self) -> bool;
}

/// The intents a typical command-driven bot needs: guild metadata, DMs, and
/// message content (for prefix commands).
pub fn default_intents() -> GatewayIntents {
    GatewayIntents::GUILDS | GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT
}

/// Builds a poise/serenity client with shared logging and error-handling
/// boilerplate. The caller supplies its own data type, error type, command
/// list, and a `setup` closure that produces the framework data once the bot is
/// connected. The returned client still needs `.start()`.
pub async fn spawn_client<Data, Err, F>(
    token: String,
    intents: GatewayIntents,
    prefix: Option<String>,
    commands: Vec<poise::Command<Data, Err>>,
    setup: F,
) -> Result<Client, serenity::Error>
where
    Data: Send + Sync + 'static,
    Err: UserFacingError + std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    F: for<'a> FnOnce(
            &'a serenity::all::Context,
            &'a serenity::all::Ready,
            &'a poise::Framework<Data, Err>,
        ) -> poise::BoxFuture<'a, Result<Data, Err>>
        + Send
        + Sync
        + 'static,
{
    let options: poise::FrameworkOptions<Data, Err> = poise::FrameworkOptions {
        commands,
        prefix_options: poise::PrefixFrameworkOptions {
            prefix,
            ..Default::default()
        },
        pre_command: |ctx| {
            Box::pin(async move {
                log::info!(
                    "Executing command {} (invoked by {})",
                    ctx.command().qualified_name,
                    ctx.author().name,
                );
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                log::info!("Executed command {}", ctx.command().qualified_name);
            })
        },
        on_error: |error| {
            Box::pin(async move {
                match error {
                    poise::FrameworkError::Command { error, ctx, .. } => {
                        log::error!("Command `{}` failed: {error}", ctx.command().qualified_name,);
                        let reply = if error.is_user_facing() {
                            format!("Error: {error}")
                        } else {
                            "Internal error — check the logs.".to_string()
                        };
                        let _ = ctx.say(reply).await;
                    }
                    poise::FrameworkError::CommandPanic { payload, ctx, .. } => {
                        log::error!(
                            "Command `{}` panicked: {payload:?}",
                            ctx.command().qualified_name,
                        );
                        let _ = ctx
                            .say("Internal error — the command panicked. Check the logs.")
                            .await;
                    }
                    poise::FrameworkError::Setup { error, .. } => {
                        log::error!("Framework setup failed: {error}");
                    }
                    poise::FrameworkError::EventHandler { error, event, .. } => {
                        log::error!(
                            "Event handler for `{}` failed: {error}",
                            event.snake_case_name(),
                        );
                    }
                    other => {
                        if let Err(e) = poise::builtins::on_error(other).await {
                            log::error!("Error while handling error: {e}");
                        }
                    }
                }
            })
        },
        ..Default::default()
    };

    let framework = poise::FrameworkBuilder::default()
        .options(options)
        .setup(setup)
        .build();

    Client::builder(token, intents).framework(framework).await
}
