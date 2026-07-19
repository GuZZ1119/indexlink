//! HTTP coverage for the read-only paper-portfolio route.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use broker::{
    BrokerClient, BrokerError, BrokerOrderAck, BrokerOrderRequest, PaperPortfolioSnapshot,
};
use indexlink_api::{build_router_with_cors, ApiState, ReadinessCheck, ReadinessError};
use rust_decimal::Decimal;
use tower::ServiceExt;

/// Readiness fixture that permits isolated portfolio route tests.
struct Ready;

#[async_trait]
impl ReadinessCheck for Ready {
    async fn check(&self) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// Broker fixture that returns one source-backed-style paper snapshot.
struct PortfolioBroker;

#[async_trait]
impl BrokerClient for PortfolioBroker {
    async fn submit_order(
        &self,
        _request: BrokerOrderRequest,
    ) -> Result<BrokerOrderAck, BrokerError> {
        Err(BrokerError::Unavailable)
    }

    async fn read_paper_portfolio(&self) -> Result<PaperPortfolioSnapshot, BrokerError> {
        Ok(PaperPortfolioSnapshot {
            currency: "USD".to_owned(),
            cash: Decimal::new(800, 0),
            buying_power: Decimal::new(800, 0),
            total_assets: Decimal::new(1_000, 0),
            market_value: Decimal::new(200, 0),
            positions: Vec::new(),
            orders: Vec::new(),
        })
    }
}

/// Verify the portfolio route returns only the normalized paper-account snapshot.
#[tokio::test]
async fn paper_portfolio_returns_configured_broker_snapshot() {
    let state =
        ApiState::with_readiness(Arc::new(Ready), "test").with_broker(Arc::new(PortfolioBroker));
    let response = build_router_with_cors(state, Vec::new())
        .oneshot(
            Request::builder()
                .uri("/paper-portfolio")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        serde_json::json!({
            "currency": "USD",
            "cash": "800",
            "buying_power": "800",
            "total_assets": "1000",
            "market_value": "200",
            "positions": [],
            "orders": []
        })
    );
}

/// Verify a broker without read support maps to the existing safe unavailable envelope.
#[tokio::test]
async fn paper_portfolio_without_reader_is_unavailable() {
    let state = ApiState::with_readiness(Arc::new(Ready), "test");
    let response = build_router_with_cors(state, Vec::new())
        .oneshot(
            Request::builder()
                .uri("/paper-portfolio")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
