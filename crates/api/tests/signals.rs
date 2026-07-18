use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState, ReadinessCheck, ReadinessError};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Ready dependency used by isolated signal-route tests.
struct Ready;

#[async_trait]
impl ReadinessCheck for Ready {
    /// Always report the test dependency as ready.
    async fn check(&self) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// Build an app without external market-data or AI dependencies.
fn app() -> axum::Router {
    build_router(ApiState::with_readiness(Arc::new(Ready), "0.1.0"))
}

/// Decode an HTTP JSON response body.
async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Produce the default five-year monthly history required by preview routes.
fn monthly_history(value: f64) -> Vec<f64> {
    vec![value; 60]
}

/// Verify the fundamental route returns audit-friendly percentile fields.
#[tokio::test]
async fn fundamental_preview_returns_computed_signal() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/signals/fundamental/preview")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "cape_history": monthly_history(20.0),
                        "cape_current": 20.0,
                        "erp_history": monthly_history(4.0),
                        "erp_current": 4.0
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({
            "score": 0.5,
            "cape_percentile": 1.0,
            "erp_percentile": 1.0
        })
    );
}

/// Verify the trend route returns its regime and raw component percentiles.
#[tokio::test]
async fn trend_preview_returns_computed_signal() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/signals/trend/preview")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "ma_distance_history": monthly_history(0.10),
                        "ma_distance_current": 0.10,
                        "rsi_history": monthly_history(70.0),
                        "rsi_current": 70.0,
                        "vix_history": monthly_history(20.0),
                        "vix_current": 20.0
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({
            "score": 0.3,
            "ma_distance_percentile": 1.0,
            "rsi_percentile": 1.0,
            "vix_percentile": 1.0,
            "regime": "falling_knife"
        })
    );
}

/// Verify invalid histories and unrecognized fields use the shared error contract.
#[tokio::test]
async fn signal_previews_reject_invalid_input() {
    for (uri, body) in [
        (
            "/signals/fundamental/preview",
            json!({
                "cape_history": [20.0],
                "cape_current": 20.0,
                "erp_history": monthly_history(4.0),
                "erp_current": 4.0
            }),
        ),
        (
            "/signals/trend/preview",
            json!({
                "ma_distance_history": monthly_history(0.10),
                "ma_distance_current": 0.10,
                "rsi_history": monthly_history(70.0),
                "rsi_current": 70.0,
                "vix_history": monthly_history(20.0),
                "vix_current": 20.0,
                "unexpected": true
            }),
        ),
    ] {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_json(response).await,
            json!({"error": {"code": "bad_request", "message": "invalid request"}})
        );
    }
}
