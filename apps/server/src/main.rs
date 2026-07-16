#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! IndexLink HTTP 服务进程。

mod config;
mod shutdown;

use ai_client::{QwenClient, RssNewsSource};
use config::Config;
use indexlink_api::{build_router_with_cors, ApiState};
use indexlink_storage::SqliteStorage;
use std::sync::Arc;
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
    let market_sentiment_configured = config.qwen.is_some();
    let state = build_api_state(storage, config.qwen);
    let app = build_router_with_cors(state, config.cors_allowed_origins);
    let listener = tokio::net::TcpListener::bind(config.address).await?;

    tracing::info!(
        address = %config.address,
        market_sentiment_configured,
        "indexlink server started"
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::signal())
        .await?;
    tracing::info!("indexlink server stopped");

    Ok(())
}

/// Assemble production API state with an optional Qwen market-sentiment pipeline.
fn build_api_state(storage: SqliteStorage, qwen: Option<ai_client::AiConfig>) -> ApiState {
    let state = ApiState::new(storage, env!("CARGO_PKG_VERSION"));
    match qwen {
        Some(qwen_config) => state.with_market_sentiment(
            Arc::new(RssNewsSource::new()),
            Arc::new(QwenClient::new(qwen_config)),
        ),
        None => state,
    }
}

fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,indexlink_server=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ai_client::AiConfig;

    use super::*;

    /// Build an isolated local storage handle for composition-root tests.
    async fn storage() -> SqliteStorage {
        SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
            .await
            .expect("in-memory SQLite storage should connect")
    }

    /// Verify the production composition root leaves sentiment unavailable without Qwen config.
    #[tokio::test]
    async fn build_api_state_leaves_market_sentiment_unconfigured_without_qwen() {
        let state = build_api_state(storage().await, None);

        assert!(format!("{state:?}").contains("market_sentiment: None"));
    }

    /// Verify the production composition root injects Qwen without exposing its API key.
    #[tokio::test]
    async fn build_api_state_injects_qwen_market_sentiment_when_configured() {
        let state = build_api_state(
            storage().await,
            Some(AiConfig {
                api_key: "server-test-secret".to_owned(),
                ..Default::default()
            }),
        );
        let debug = format!("{state:?}");

        assert!(debug.contains("market_sentiment: Some(MarketSentimentDependencies)"));
        assert!(!debug.contains("server-test-secret"));
    }
}
