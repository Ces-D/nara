#[tokio::main]
async fn main() {
    env_logger::init();

    let token = std::env::var("TITANS_TOWER_DISCORD_BOT_TOKEN")
        .expect("Missing TITANS_TOWER_DISCORD_BOT_TOKEN");

    match titans_tower::spawn_client(token).await {
        Ok(mut client) => match client.start().await {
            Ok(_) => {
                log::info!("Teen Titans Go!")
            }
            Err(e) => {
                log::error!("Error message from titans_tower discord bot: {:?}", e)
            }
        },
        Err(e) => {
            log::error!("Failed to spawn titans_tower discord bot: {:?}", e)
        }
    }
}
