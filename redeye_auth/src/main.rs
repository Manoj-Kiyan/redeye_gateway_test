pub mod api;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod usecases;

use infrastructure::db::setup_db_pool;
use api::router::create_router;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "redeye_auth=debug,axum=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    if let Err(e) = run().await {
        eprintln!("Fatal error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting redeye_auth service on PORT 8084");

    // Setup SQLx DB Pool
    let pool = setup_db_pool().await
        .map_err(|e| format!("Failed to setup database pool: {}", e))?;

    tracing::info!("Running SQLx database migrations");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Migration failed: {}", e);
            format!("Database migration error: {}", e)
        })?;

    let state = AppState { db_pool: pool };

    let app = create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8084));
    tracing::debug!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await
        .map_err(|e| format!("Failed to bind TCP listener: {}", e))?;
    
    axum::serve(listener, app).await
        .map_err(|e| format!("Server error: {}", e).into())
}
