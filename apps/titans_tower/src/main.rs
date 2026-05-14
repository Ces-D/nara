use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post},
};
use std::net::SocketAddr;

mod db;
mod discord;
mod error;
mod ops;
mod service;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(e) = run().await {
        eprintln!("titans_tower: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), error::ServiceError> {
    let token = std::env::var("TITANS_TOWER_DISCORD_BOT_TOKEN").map_err(|_| {
        error::ServiceError::Config("missing TITANS_TOWER_DISCORD_BOT_TOKEN".into())
    })?;

    let bind_addr_str =
        std::env::var("TITANS_TOWER_BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());
    let bind_addr: SocketAddr = bind_addr_str.parse().map_err(|e| {
        error::ServiceError::Config(format!(
            "invalid TITANS_TOWER_BIND_ADDR `{bind_addr_str}`: {e}"
        ))
    })?;

    let konan_pool = konan_core::print_ops::pool().map_err(error::ServiceError::from)?;
    let brainiac_pool =
        brainiac_core::database::connection::pool().map_err(error::ServiceError::from)?;

    let konan_routes = Router::new()
        .route("/print/outline", post(service::konan::print_outline))
        .route("/print/tracker", post(service::konan::print_tracker))
        .route("/print/file", post(service::konan::print_file))
        .route(
            "/upload",
            post(service::konan::upload_file)
                .layer(DefaultBodyLimit::max(service::konan::MAX_UPLOAD_BYTES)),
        )
        .route(
            "/schedules",
            post(service::konan::create_print_schedule)
                .get(service::konan::list_scheduled_print_tasks),
        )
        .route(
            "/schedules/{id}",
            delete(service::konan::delete_scheduled_print_task),
        )
        .with_state(konan_pool.clone());

    let brainiac_routes = Router::new()
        .route(
            "/categories",
            get(service::brainiac::list_categories).post(service::brainiac::create_category),
        )
        .route(
            "/categories/{id}",
            delete(service::brainiac::delete_category),
        )
        .route(
            "/category-tags",
            post(service::brainiac::add_category_tag)
                .delete(service::brainiac::remove_category_tag),
        )
        .route("/tags", get(service::brainiac::list_tags))
        .route("/practice", post(service::brainiac::practice))
        .route(
            "/practice/items",
            post(service::brainiac::create_practice_items),
        )
        .route(
            "/practice/items/{id}",
            patch(service::brainiac::edit_practice_item),
        )
        .route(
            "/practice/items/{id}/answer",
            get(service::brainiac::practice_item_answer),
        )
        .with_state(brainiac_pool.clone());

    let app = Router::new()
        .nest("/konan", konan_routes)
        .nest("/brainiac", brainiac_routes);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    log::info!("HTTP listening on {bind_addr}");

    let mut client = discord::spawn_client(token, konan_pool, brainiac_pool).await?;

    let bot = async move { client.start().await.map_err(error::ServiceError::from) };
    let server = async move {
        axum::serve(listener, app)
            .await
            .map_err(error::ServiceError::from)
    };

    tokio::try_join!(bot, server)?;
    Ok(())
}
