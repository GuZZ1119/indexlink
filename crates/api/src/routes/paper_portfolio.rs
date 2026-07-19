//! Read-only paper-portfolio routes.

use axum::{extract::State, routing::get, Json, Router};
use broker::PaperPortfolioSnapshot;

use crate::{ApiError, ApiState};

/// Build paper-account overview routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new().route("/paper-portfolio", get(read_paper_portfolio))
}

/// Read the configured local OpenD paper account without placing an order.
async fn read_paper_portfolio(
    State(state): State<ApiState>,
) -> Result<Json<PaperPortfolioSnapshot>, ApiError> {
    Ok(Json(state.paper_portfolio().await?))
}
