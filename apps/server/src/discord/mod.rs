use crate::error::ServiceError;
use serenity::prelude::Client;

mod commands;
mod ui;

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
    titans_tower::spawn_client(
        token,
        titans_tower::default_intents(),
        Some("-".into()),
        vec![commands::konan::konan(), commands::brainiac::brainiac()],
        move |ctx, ready, framework| {
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
        },
    )
    .await
}
