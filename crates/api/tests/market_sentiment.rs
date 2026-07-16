use std::sync::Arc;

use ai_client::{AiClientError, AiProvider, NewsItem, NewsSource, NewsSourceError, Sentiment};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState, ReadinessCheck, ReadinessError};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Ready dependency used by isolated route tests.
struct Ready;

#[async_trait]
impl ReadinessCheck for Ready {
    /// Always report the test dependency as ready.
    async fn check(&self) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// Deterministic news source for HTTP route tests.
struct StaticNews;

#[async_trait]
impl NewsSource for StaticNews {
    /// Return one representative news item without network access.
    async fn fetch(&self) -> Result<Vec<NewsItem>, NewsSourceError> {
        Ok(vec![NewsItem {
            title: "Markets rise on improving inflation data".to_owned(),
            description: "A compact deterministic test item.".to_owned(),
            pub_date: Utc::now(),
        }])
    }
}

/// AI provider that returns a fixed positive signal.
struct PositiveAi;

#[async_trait]
impl AiProvider for PositiveAi {
    /// Return a bounded sentiment without network access.
    async fn analyze(&self, _prompt: &str) -> Result<Sentiment, AiClientError> {
        Ok(Sentiment::new(0.4).expect("constant sentiment is in range"))
    }
}

/// AI provider that simulates a private provider failure.
struct FailingAi;

#[async_trait]
impl AiProvider for FailingAi {
    /// Return a provider error whose internal details must not reach HTTP clients.
    async fn analyze(&self, _prompt: &str) -> Result<Sentiment, AiClientError> {
        Err(AiClientError::EmptyResponse)
    }
}

/// Build an app with an optional, fully injected market-sentiment pipeline.
fn app(provider: Option<Arc<dyn AiProvider>>) -> axum::Router {
    let state = ApiState::with_readiness(Arc::new(Ready), "0.1.0");
    let state = match provider {
        Some(provider) => state.with_market_sentiment(Arc::new(StaticNews), provider),
        None => state,
    };
    build_router(state)
}

/// Decode an HTTP JSON response body.
async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Verify a fake AI provider is injected and produces the documented response.
#[tokio::test]
async fn preview_returns_sentiment_from_injected_provider() {
    let response = app(Some(Arc::new(PositiveAi)))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/market-sentiment/preview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"score": 0.4, "label": "positive"})
    );
}

/// Verify a missing provider follows the standard unavailable JSON contract.
#[tokio::test]
async fn preview_without_configured_provider_is_unavailable() {
    let response = app(None)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/market-sentiment/preview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response_json(response).await,
        json!({
            "error": {
                "code": "service_unavailable",
                "message": "service is unavailable"
            }
        })
    );
}

/// Verify provider errors are mapped without exposing provider internals.
#[tokio::test]
async fn preview_provider_error_uses_safe_unavailable_response() {
    let response = app(Some(Arc::new(FailingAi)))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/market-sentiment/preview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response_json(response).await;
    assert_eq!(body["error"]["code"], json!("service_unavailable"));
    assert_eq!(body["error"]["message"], json!("service is unavailable"));
    assert!(!body.to_string().contains("EmptyResponse"));
}
