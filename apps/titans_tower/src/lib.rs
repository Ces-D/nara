use serenity::prelude::{Client, GatewayIntents};

mod commands;
mod ui;

pub type Context<'a> = poise::Context<'a, AppData, AppError>;

pub struct AppData {
    pub konan_pool: konan_core::print_ops::KonanDbPool,
    pub brainiac_pool: brainiac_core::database::connection::BrainiacDbPool,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("discord error: {0}")]
    Serenity(#[from] serenity::Error),
    #[error("invalid input: {0}")]
    Field(#[from] ui::FieldParseError),
    #[error("print operation: {0}")]
    PrintOperation(#[from] konan_core::print_ops::KonanDbError),
    #[error("brainiac operation: {0}")]
    BrainiacOperation(#[from] brainiac_core::database::BrainiacDbError),
    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

pub async fn spawn_client(token: String) -> Result<Client, serenity::Error> {
    let options: poise::FrameworkOptions<AppData, AppError> = poise::FrameworkOptions {
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
                        let _ = ctx.say(format!("Error: {error}")).await;
                    }
                    poise::FrameworkError::CommandPanic { payload, ctx, .. } => {
                        log::error!(
                            "Command `{}` panicked: {payload:?}",
                            ctx.command().qualified_name,
                        );
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
    let konan_pool =
        konan_core::print_ops::pool().expect("Failed to open konan database connection");
    let brainiac_pool = brainiac_core::database::connection::pool()
        .expect("Failed to open brainiac database connection");

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
                    konan_pool,
                    brainiac_pool,
                })
            })
        })
        .build();

    let intents =
        GatewayIntents::GUILDS | GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    Client::builder(token, intents).framework(framework).await
}
