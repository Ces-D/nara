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
        commands: vec![
            commands::konan::konan(),
            commands::brainiac::brainiac(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("-".into()),
            ..Default::default()
        },
        pre_command: |ctx| {
            Box::pin(async move {
                log::trace!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                log::trace!("Executed command {}!", ctx.command().qualified_name);
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
        .setup(|_ctx, _ready, _framework| {
            Box::pin(async move {
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
