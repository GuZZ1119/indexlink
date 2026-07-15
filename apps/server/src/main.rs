#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! IndexLink HTTP 服务进程。

mod config;
mod shutdown;

use config::Config;
use indexlink_api::{build_router_with_cors, ApiState};
use indexlink_storage::SqliteStorage;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    init_tracing()?;

    let config = Config::from_env()?;
    let storage = SqliteStorage::connect_with_options(
        &config.database_url,
        config.database_max_connections,
        config.database_connect_timeout,
    )
    .await?;
    storage.migrate().await?;
    tracing::info!("SQLite migrations applied");
    let state = ApiState::new(storage, env!("CARGO_PKG_VERSION"));
    let app = build_router_with_cors(state, config.cors_allowed_origins);
    let listener = tokio::net::TcpListener::bind(config.address).await?;

    tracing::info!(address = %config.address, "indexlink server started");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::signal())
        .await?;
    tracing::info!("indexlink server stopped");

    Ok(())
}

fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,indexlink_server=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()
}
