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

mod discord;
mod error;
mod features;
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
    log::info!("nara server: starting up");

    let token = std::env::var("NARA_SERVER_DISCORD_BOT_TOKEN")
        .map_err(|_| error::ServiceError::Config("missing NARA_SERVER_DISCORD_BOT_TOKEN".into()))?;

    let bind_addr_str =
        std::env::var("NARA_SERVER_BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());
    let bind_addr: SocketAddr = bind_addr_str.parse().map_err(|e| {
        error::ServiceError::Config(format!(
            "invalid NARA_SERVER_BIND_ADDR `{bind_addr_str}`: {e}"
        ))
    })?;
    log::debug!("nara server: config loaded, bind_addr={bind_addr}");

    let allowed_origins = parse_allowed_origins()?;
    log::debug!(
        "nara server: {} allowed CORS origin(s)",
        allowed_origins.len()
    );

    log::info!("nara server: initializing brainiac database pool");
    let brainiac_pool =
        brainiac_core::database::connection::pool().map_err(error::ServiceError::from)?;
    log::info!("nara server: initializing cadence database pool");
    let cadence_pool = cadence_core::database::pool()?;
    let konan = konan_core::KonanScheduler::new(cadence_pool.clone());

    log::info!("nara server: initializing bean database pool");
    let bean_pool = bean::pool()?;

    log::info!("nara server: spawning discord client");
    let mut client = discord::spawn_client(
        token,
        konan.clone(),
        brainiac_pool.clone(),
        bean_pool.clone(),
    )
    .await?;
    let discord_http = client.http.clone();

    log::debug!("nara server: registering task handlers and channels");
    let mut tasks = cadence_core::registry::TaskRegistry::default();
    konan_core::KonanScheduler::register_handlers(&mut tasks);
    features::daily_dilly::register(&mut tasks, discord_http.clone(), bean_pool.clone())?;
    features::daily_dilly::ensure_schedule(&cadence_pool).await?;

    let mut channels = cadence_core::channels::ChannelRegistry::default();
    konan_core::KonanScheduler::register_channels(&mut channels);
    titans_tower::register_channels(&mut channels, discord_http)?;

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

    let bot = async move {
        log::info!("nara server: discord bot loop started");
        client.start().await.map_err(error::ServiceError::from)
    };
    let server = async move {
        axum::serve(listener, app)
            .await
            .map_err(error::ServiceError::from)
    };
    let executor_pool = cadence_pool.clone();
    let executor = async move {
        log::info!("nara server: cadence executor loop started");
        cadence_core::executor::run(executor_pool, tasks, channels).await;
        Ok::<(), error::ServiceError>(())
    };

    log::info!("nara server: all subsystems initialized, entering run loop");
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
