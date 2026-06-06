use crate::error::ServiceError;
use serenity::prelude::Client;

mod commands;
mod ui;

pub type Context<'a> = poise::Context<'a, AppData, ServiceError>;

pub struct AppData {
    pub konan: konan_core::KonanScheduler,
    pub brainiac_pool: brainiac_core::database::connection::BrainiacDbPool,
    // Available to Discord commands; today the bean pool is consumed by the
    // daily-dilly cadence handler (registered in main), so no command reads it
    // through AppData yet.
    #[allow(dead_code)]
    pub bean_pool: bean::database::BeanDBPool,
}

pub async fn spawn_client(
    token: String,
    konan: konan_core::KonanScheduler,
    brainiac_pool: brainiac_core::database::connection::BrainiacDbPool,
    bean_pool: bean::database::BeanDBPool,
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

                // Refresh each command's pinned help message in its dedicated
                // channel. Channels are resolved by id from NARA_DISCORD_CHANNELS
                // (owner-provisioned); a missing config entry or unreachable
                // channel is fatal.
                let bot_id = ready.user.id;
                let channel_specs = [
                    (commands::konan::CHANNEL, commands::konan::help_message()),
                    (
                        commands::brainiac::CHANNEL,
                        commands::brainiac::help_message(),
                    ),
                    (
                        crate::features::daily_dilly::CHANNEL,
                        crate::features::daily_dilly::help_message(),
                    ),
                ];
                for (name, help) in &channel_specs {
                    let channel_id = titans_tower::configured_channel_id(name)?;
                    let channel = channel_id.to_channel(&ctx.http).await?.guild().ok_or_else(
                        || {
                            ServiceError::Config(format!(
                                "configured channel `{name}` ({channel_id}) is not a guild channel"
                            ))
                        },
                    )?;
                    titans_tower::refresh_pinned_message(&ctx.http, &channel, bot_id, help).await?;
                }

                Ok(AppData {
                    konan,
                    brainiac_pool,
                    bean_pool,
                })
            })
        },
    )
    .await
}
