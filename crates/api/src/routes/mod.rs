mod decision_preview;
mod decision_records;
mod health;
mod investment_plans;
mod market_sentiment;
mod ready;

use axum::{routing::get, Router};

use crate::ApiState;

pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(ready::ready))
        .merge(decision_preview::router())
        .merge(decision_records::router())
        .merge(investment_plans::router())
        .merge(market_sentiment::router())
}
