mod decision_preview;
mod decision_records;
mod health;
mod investment_plans;
mod market_data;
mod market_sentiment;
mod paper_portfolio;
mod ready;
mod signals;

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
        .merge(market_data::router())
        .merge(paper_portfolio::router())
        .merge(signals::router())
}
