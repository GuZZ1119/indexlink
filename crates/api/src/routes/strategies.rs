//! Read-only HTTP routes for persisted restricted DSL strategy versions.

use axum::{
    extract::{rejection::PathRejection, Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use indexlink_storage::StoredStrategySpec;
use serde::Serialize;
use strategy_dsl::StrategySpecDocument;
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};

use crate::{ApiError, ApiState};

/// A safe validation result for one form-authored restricted DSL strategy.
#[derive(Debug, Serialize)]
struct StrategyValidationResponse {
    /// Whether the submitted document rebuilt into a validated immutable strategy.
    valid: bool,
    /// Human-readable validation failure without transport, database, or credential details.
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    /// Canonical validated document, returned only when validation succeeds.
    #[serde(skip_serializing_if = "Option::is_none")]
    document: Option<StrategySpecDocument>,
}

/// Build restricted strategy discovery, validation, and immutable-save routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/strategies", get(list_strategies).post(create_strategy))
        .route("/strategies/validate", post(validate_strategy))
        .route("/strategies/:policy_id/:policy_version", get(get_strategy))
}

/// Validate one form-authored restricted DSL document without persisting it.
async fn validate_strategy(
    Json(document): Json<StrategySpecDocument>,
) -> Json<StrategyValidationResponse> {
    match document.into_strategy_spec() {
        Ok(strategy) => Json(StrategyValidationResponse {
            valid: true,
            error: None,
            document: Some(StrategySpecDocument::from_strategy_spec(&strategy)),
        }),
        Err(error) => Json(StrategyValidationResponse {
            valid: false,
            error: Some(error.to_string()),
            document: None,
        }),
    }
}

/// Persist one new immutable validated DSL strategy version.
async fn create_strategy(
    State(state): State<ApiState>,
    Json(document): Json<StrategySpecDocument>,
) -> Result<(StatusCode, Json<StoredStrategySpec>), ApiError> {
    let strategy = document
        .into_strategy_spec()
        .map_err(|_| ApiError::BadRequest)?;
    Ok((
        StatusCode::CREATED,
        Json(state.save_strategy_spec(&strategy).await?),
    ))
}

/// List all immutable persisted DSL strategy versions.
async fn list_strategies(
    State(state): State<ApiState>,
) -> Result<Json<Vec<StoredStrategySpec>>, ApiError> {
    Ok(Json(state.list_strategy_specs().await?))
}

/// Fetch one immutable persisted DSL strategy version by its policy reference.
async fn get_strategy(
    State(state): State<ApiState>,
    path: Result<Path<(String, u32)>, PathRejection>,
) -> Result<Json<StoredStrategySpec>, ApiError> {
    let Path((policy_id, policy_version)) = path.map_err(|_| ApiError::BadRequest)?;
    let policy = PolicyRef::new(
        PolicyId::new(policy_id).map_err(|_| ApiError::BadRequest)?,
        PolicyVersion::new(policy_version).map_err(|_| ApiError::BadRequest)?,
    );
    Ok(Json(state.get_strategy_spec(&policy).await?))
}
