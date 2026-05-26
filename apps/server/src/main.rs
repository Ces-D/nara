use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, StatusCode, header},
    routing::{delete, get, patch, post},
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

mod db;
mod discord;
mod error;
mod ops;
mod service;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CORS_MAX_AGE: Duration = Duration::from_secs(3600);

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(e) = run().await {
        eprintln!("nara server: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), error::ServiceError> {
    let token = std::env::var("NARA_SERVER_DISCORD_BOT_TOKEN")
        .map_err(|_| error::ServiceError::Config("missing NARA_SERVER_DISCORD_BOT_TOKEN".into()))?;

    let bind_addr_str =
        std::env::var("NARA_SERVER_BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());
    let bind_addr: SocketAddr = bind_addr_str.parse().map_err(|e| {
        error::ServiceError::Config(format!(
            "invalid NARA_SERVER_BIND_ADDR `{bind_addr_str}`: {e}"
        ))
    })?;

    let allowed_origins = parse_allowed_origins()?;

    let brainiac_pool =
        brainiac_core::database::connection::pool().map_err(error::ServiceError::from)?;
    let cadence_pool = cadence_core::database::pool()?;
    let konan = konan_core::KonanScheduler::new(cadence_pool.clone());

    let mut tasks = cadence_core::registry::TaskRegistry::default();
    konan_core::KonanScheduler::register_handlers(&mut tasks);

    let mut channels = cadence_core::channels::ChannelRegistry::default();
    konan_core::KonanScheduler::register_channels(&mut channels);

    let tasks = Arc::new(tasks);
    let channels = Arc::new(channels);

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
        .with_state(konan.clone());

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

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE])
        .max_age(CORS_MAX_AGE)
        .allow_origin(allowed_origins);

    let app = Router::new()
        .nest("/konan", konan_routes)
        .nest("/brainiac", brainiac_routes)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    log::info!("HTTP listening on {bind_addr}");

    let mut client = discord::spawn_client(token, konan, brainiac_pool).await?;

    let bot = async move { client.start().await.map_err(error::ServiceError::from) };
    let server = async move {
        axum::serve(listener, app)
            .await
            .map_err(error::ServiceError::from)
    };
    let executor_pool = cadence_pool.clone();
    let executor = async move {
        cadence_core::executor::run(executor_pool, tasks, channels).await;
        Ok::<(), error::ServiceError>(())
    };

    tokio::try_join!(bot, server, executor)?;
    Ok(())
}

fn parse_allowed_origins() -> Result<Vec<HeaderValue>, error::ServiceError> {
    std::env::var("NARA_SERVER_ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|origin| {
            origin.parse::<HeaderValue>().map_err(|e| {
                error::ServiceError::Config(format!(
                    "invalid origin `{origin}` in NARA_SERVER_ALLOWED_ORIGINS: {e}"
                ))
            })
        })
        .collect()
}
