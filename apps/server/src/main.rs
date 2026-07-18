#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! IndexLink HTTP 服务进程。

mod config;
mod shutdown;

use ai_client::{QwenClient, RssNewsSource};
use broker::{
    BrokerClient, BrokerError, OpenDConnectionConfig, OpenDPaperBroker, OpenDPaperSession,
    OpenDSessionError,
};
use config::Config;
use indexlink_api::{build_router_with_cors, ApiState};
use indexlink_storage::SqliteStorage;
use std::{future::Future, sync::Arc};
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
    let paper_broker_configured = config.opend.is_some();
    let state =
        build_api_state(storage, config.qwen, config.opend, build_opend_paper_broker).await?;
    let app = build_router_with_cors(state, config.cors_allowed_origins);
    let listener = tokio::net::TcpListener::bind(config.address).await?;

    tracing::info!(
        address = %config.address,
        market_sentiment_configured,
        paper_broker_configured,
        "indexlink server started"
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::signal())
        .await?;
    tracing::info!("indexlink server stopped");

    Ok(())
}

/// Assemble production API state with optional Qwen and OpenD paper dependencies.
///
/// Without an OpenD configuration, the state keeps its local paper-only mock broker.
/// A configured OpenD session must initialize successfully before the server starts.
async fn build_api_state<F, Fut>(
    storage: SqliteStorage,
    qwen: Option<ai_client::AiConfig>,
    opend: Option<OpenDConnectionConfig>,
    build_broker: F,
) -> Result<ApiState, BrokerSetupError>
where
    F: FnOnce(OpenDConnectionConfig) -> Fut,
    Fut: Future<Output = Result<Arc<dyn BrokerClient>, BrokerSetupError>>,
{
    let state = ApiState::new(storage, env!("CARGO_PKG_VERSION"));
    let state = match qwen {
        Some(qwen_config) => state.with_market_sentiment(
            Arc::new(RssNewsSource::new()),
            Arc::new(QwenClient::new(qwen_config)),
        ),
        None => state,
    };
    match opend {
        Some(config) => Ok(state.with_broker(build_broker(config).await?)),
        None => Ok(state),
    }
}

/// Connect and wrap the configured local OpenD session as the production paper broker.
async fn build_opend_paper_broker(
    config: OpenDConnectionConfig,
) -> Result<Arc<dyn BrokerClient>, BrokerSetupError> {
    let session = OpenDPaperSession::connect(&config)
        .await
        .map_err(BrokerSetupError::Session)?;
    let broker = OpenDPaperBroker::new(config, session).map_err(BrokerSetupError::Adapter)?;

    Ok(Arc::new(broker))
}

/// Safe startup error when an explicitly configured OpenD paper adapter cannot initialize.
#[derive(Debug, thiserror::Error)]
enum BrokerSetupError {
    /// The local OpenD session could not be initialized.
    #[error("configured OpenD paper broker is unavailable")]
    Session(#[source] OpenDSessionError),
    /// The validated OpenD session could not become a paper broker adapter.
    #[error("configured OpenD paper broker could not be initialized")]
    Adapter(#[source] BrokerError),
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
    use std::{env, time::Duration};

    use ai_client::AiConfig;
    use async_trait::async_trait;
    use axum::{
        body::{to_bytes, Body},
        http::{header::CONTENT_TYPE, Request, StatusCode},
        response::Response,
        Router,
    };
    use broker::{BrokerOrderAck, BrokerOrderRequest, BrokerProvider};
    use serde_json::{json, Value};
    use tower::ServiceExt;

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
        let state = build_api_state(storage().await, None, None, build_opend_paper_broker)
            .await
            .expect("mock broker composition should be infallible");

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
            None,
            build_opend_paper_broker,
        )
        .await
        .expect("Qwen-only composition should be infallible");
        let debug = format!("{state:?}");

        assert!(debug.contains("market_sentiment: Some(MarketSentimentDependencies)"));
        assert!(!debug.contains("server-test-secret"));
    }

    /// Broker double used to prove the composition root replaces its default mock.
    #[derive(Debug)]
    struct UnavailableBroker;

    #[async_trait]
    impl BrokerClient for UnavailableBroker {
        async fn submit_order(
            &self,
            _request: BrokerOrderRequest,
        ) -> Result<BrokerOrderAck, BrokerError> {
            Err(BrokerError::Unavailable)
        }
    }

    /// Build a validated paper configuration without contacting its local endpoint.
    fn paper_config() -> OpenDConnectionConfig {
        OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", 11111)
            .expect("paper configuration should be valid")
    }

    /// Send a due decision-preview request with a paper order through an app router.
    async fn submit_decision_preview(
        app: Router,
        symbol: &str,
        quantity: &str,
        idempotency_key: &str,
    ) -> Response {
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/investment-plans")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "name": "OpenD paper smoke",
                            "symbol": symbol,
                            "base_contribution": "100.00",
                            "currency": "USD",
                            "schedule_kind": "monthly",
                            "schedule_day": 15,
                            "max_single_execution": "100.00"
                        })
                        .to_string(),
                    ))
                    .expect("create request should build"),
            )
            .await
            .expect("create route should respond");
        assert_eq!(created.status(), StatusCode::CREATED);
        let created = response_json(created).await;
        let plan_id = created["id"]
            .as_str()
            .expect("created plan should have an ID");

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{plan_id}/decision-preview"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "day_of_month": 15,
                        "fundamental": {
                            "score": 0.5,
                            "cape_percentile": 0.5,
                            "erp_percentile": 0.5
                        },
                        "trend": {
                            "score": 0.5,
                            "ma_distance_percentile": 0.5,
                            "rsi_percentile": 0.5,
                            "vix_percentile": 0.5,
                            "regime": "neutral"
                        },
                        "sentiment": {"score": 0.0},
                        "paper_order": {
                            "idempotency_key": idempotency_key,
                            "side": "buy",
                            "order_type": "market",
                            "quantity": quantity
                        }
                    })
                    .to_string(),
                ))
                .expect("decision request should build"),
        )
        .await
        .expect("decision route should respond")
    }

    /// Verify a configured OpenD factory failure prevents server composition.
    #[tokio::test]
    async fn build_api_state_returns_session_error_when_opend_factory_fails() {
        let error = build_api_state(storage().await, None, Some(paper_config()), |_| async {
            Err::<Arc<dyn BrokerClient>, _>(BrokerSetupError::Session(
                OpenDSessionError::Unavailable,
            ))
        })
        .await
        .expect_err("failed OpenD factory must prevent startup");

        assert!(matches!(
            error,
            BrokerSetupError::Session(OpenDSessionError::Unavailable)
        ));
    }

    /// Verify a configured broker factory replaces the default mock in the HTTP route.
    #[tokio::test]
    async fn build_api_state_uses_configured_broker_factory() {
        let storage = storage().await;
        storage
            .migrate()
            .await
            .expect("in-memory SQLite migrations should apply");
        let state = build_api_state(storage, None, Some(paper_config()), |_| async {
            Ok::<Arc<dyn BrokerClient>, BrokerSetupError>(Arc::new(UnavailableBroker))
        })
        .await
        .expect("configured factory should compose");
        let response = submit_decision_preview(
            build_router_with_cors(state, Vec::new()),
            "VOO",
            "1.00",
            "configured-broker-factory-test",
        )
        .await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    /// Read an explicitly supplied smoke value without adding a default real order.
    fn smoke_value(name: &str) -> String {
        env::var(name).unwrap_or_else(|_| panic!("{name} must be set for the real OpenD smoke"))
    }

    /// Decode a JSON HTTP response in the manual real-OpenD smoke test.
    async fn response_json(response: axum::response::Response) -> Value {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("smoke response body should be readable");
        serde_json::from_slice(&body).expect("smoke response should be JSON")
    }

    /// Submit one explicitly confirmed order through the production OpenD server wiring.
    ///
    /// This test is ignored by default because it creates a real virtual-account
    /// order. It requires a locally authenticated OpenD process, `OPEND_PROVIDER`,
    /// an explicit paper-account selection, a unique idempotency key, symbol,
    /// quantity, and `OPEND_SMOKE_CONFIRM=submit-paper-order`.
    #[tokio::test]
    #[ignore = "requires an explicitly confirmed local OpenD paper order"]
    async fn real_opend_paper_order_smoke() {
        assert_eq!(
            env::var("OPEND_SMOKE_CONFIRM").as_deref(),
            Ok("submit-paper-order"),
            "set OPEND_SMOKE_CONFIRM=submit-paper-order to acknowledge a real paper order"
        );
        let config = Config::from_env().expect("server configuration should be valid");
        let opend = config
            .opend
            .expect("OPEND_PROVIDER must configure the real paper broker");
        assert!(
            opend.account_id().is_some(),
            "OPEND_ACCOUNT_ID must explicitly select one paper account for the smoke"
        );
        let symbol = smoke_value("OPEND_SMOKE_SYMBOL");
        let quantity = smoke_value("OPEND_SMOKE_QUANTITY");
        let idempotency_key = smoke_value("OPEND_SMOKE_IDEMPOTENCY_KEY");
        let storage = storage().await;
        storage
            .migrate()
            .await
            .expect("in-memory SQLite migrations should apply");
        let app = build_router_with_cors(
            build_api_state(storage, None, Some(opend), build_opend_paper_broker)
                .await
                .expect("local OpenD paper broker should initialize"),
            Vec::new(),
        );
        let response = submit_decision_preview(app, &symbol, &quantity, &idempotency_key).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = response_json(response).await;

        assert_eq!(response["paper_order_ack"]["environment"], "paper");
        assert_eq!(response["paper_order_ack"]["status"], "accepted");
        assert!(response["paper_order_ack"]["order_id"].is_string());
    }
}
