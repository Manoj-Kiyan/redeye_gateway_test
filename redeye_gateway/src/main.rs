//! main.rs - RedEye Gateway entry point (Clean Architecture).
//! Bootstrap only: load config, init tracing, build state, start server.

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use dotenvy::dotenv;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod api;
mod config;
mod domain;
mod infrastructure;
mod usecases;

use config::GatewayConfig;
use domain::models::AppState;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    if let Err(error) = run().await {
        eprintln!("Fatal error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = GatewayConfig::from_env().map_err(|error| format!("Configuration error: {error}"))?;

    let redis_config = deadpool_redis::Config::from_url(&config.redis_url);
    let redis_pool = redis_config
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .map_err(|error| format!("Failed to create Redis connection pool: {error}"))?;

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|error| format!("Failed to construct reqwest HTTP client: {error}"))?;

    let db_pool = sqlx::PgPool::connect(&config.database_url)
        .await
        .map_err(|error| format!("Failed to connect to Postgres DB: {error}"))?;

    let state = Arc::new(AppState {
        http_client,
        redis_pool,
        db_pool,
        config: config.clone(),
    });

    let app: Router = api::routes::create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    info!(port = config.port, "RedEye Gateway listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|error| format!("Failed to bind TCP listener: {error}"))?;

    axum::serve(listener, app)
        .await
        .map_err(|error| format!("Axum server encountered a fatal error: {error}").into())
}
