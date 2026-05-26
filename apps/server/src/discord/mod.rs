use crate::error::ServiceError;
use serenity::prelude::{Client, GatewayIntents};

mod commands;
mod ui;

pub use ui::FieldParseError;

pub type Context<'a> = poise::Context<'a, AppData, ServiceError>;

pub struct AppData {
    pub konan: konan_core::KonanScheduler,
    pub brainiac_pool: brainiac_core::database::connection::BrainiacDbPool,
}

pub async fn spawn_client(
    token: String,
    konan: konan_core::KonanScheduler,
    brainiac_pool: brainiac_core::database::connection::BrainiacDbPool,
) -> Result<Client, serenity::Error> {
    let options: poise::FrameworkOptions<AppData, ServiceError> = poise::FrameworkOptions {
        commands: vec![commands::konan::konan(), commands::brainiac::brainiac()],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("-".into()),
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
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                log::info!("Connected to Discord as {}", ready.user.name);
                let commands = &framework.options().commands;
                poise::builtins::register_globally(ctx, commands).await?;
                log::info!(
                    "Registered {} top-level slash commands globally",
                    commands.len()
                );
                Ok(AppData {
                    konan,
                    brainiac_pool,
                })
            })
        })
        .build();

    let intents =
        GatewayIntents::GUILDS | GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    Client::builder(token, intents).framework(framework).await
}
