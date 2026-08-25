//! Read-only HTTP routes for persisted restricted DSL strategy versions.

use axum::{
    extract::{rejection::PathRejection, Path, State},
    routing::get,
    Json, Router,
};
use indexlink_storage::StoredStrategySpec;
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};

use crate::{ApiError, ApiState};

/// Build read-only strategy discovery routes.
///
/// This PR intentionally has no create, update, activation, evaluation, or order route. A
/// stored strategy remains inert until a later reviewed activation flow exists.
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/strategies", get(list_strategies))
        .route("/strategies/:policy_id/:policy_version", get(get_strategy))
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
