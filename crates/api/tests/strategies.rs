use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use core_domain::Multiplier;
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState};
use indexlink_storage::{SqliteStorage, SqliteStrategySpecRepository};
use rust_decimal::Decimal;
use serde_json::Value;
use strategy_dsl::{
    ComparisonOperator, Condition, IndicatorSpec, LookbackWindow, PolicyAction, StrategyRule,
    StrategySpec, ValueExpression,
};
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};
use tower::ServiceExt;

/// Build a valid immutable strategy version for read-only API tests.
fn strategy() -> StrategySpec {
    StrategySpec::new(
        PolicyRef::new(
            PolicyId::new("dsl_api_test").unwrap(),
            PolicyVersion::new(1).unwrap(),
        ),
        "API RSI guard",
        vec![StrategyRule::new(
            Condition::compare(
                ValueExpression::indicator(IndicatorSpec::RelativeStrengthIndex(
                    LookbackWindow::new(14).unwrap(),
                )),
                ComparisonOperator::LessThan,
                Decimal::new(35, 0),
            ),
            PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.1)),
        )],
    )
    .unwrap()
}

/// Read a JSON HTTP response without leaking internal repository errors into assertions.
async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Verify stored strategy versions are discoverable but not mutable through this API surface.
#[tokio::test]
async fn lists_and_reads_persisted_strategy_versions() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    SqliteStrategySpecRepository::new(storage.pool().clone())
        .save(&strategy())
        .await
        .unwrap();
    let app = build_router(ApiState::new(storage, "0.1.0"));

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/strategies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = response_json(listed).await;
    assert_eq!(listed[0]["policy"]["id"], "dsl_api_test");
    assert_eq!(listed[0]["policy"]["version"], 1);
    assert_eq!(listed[0]["document"]["rules"].as_array().unwrap().len(), 1);

    let fetched = app
        .oneshot(
            Request::builder()
                .uri("/strategies/dsl_api_test/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fetched.status(), StatusCode::OK);
    assert_eq!(response_json(fetched).await["name"], "API RSI guard");
}

/// Verify malformed or absent policy references use the established safe error envelope.
#[tokio::test]
async fn rejects_invalid_or_unknown_strategy_references() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    let app = build_router(ApiState::new(storage, "0.1.0"));

    let invalid = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/strategies/fixed-dca/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let unknown = app
        .oneshot(
            Request::builder()
                .uri("/strategies/dsl_missing/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
}
