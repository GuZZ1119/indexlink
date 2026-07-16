//! Market-sentiment preview HTTP route.

use ai_client::Sentiment;
use axum::{extract::State, routing::post, Json, Router};
use serde::Serialize;

use crate::{ApiError, ApiState};

/// Build market-sentiment preview routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new().route("/market-sentiment/preview", post(preview_market_sentiment))
}

/// Fetch current market news and derive one Qwen sentiment score.
async fn preview_market_sentiment(
    State(state): State<ApiState>,
) -> Result<Json<MarketSentimentResponse>, ApiError> {
    let sentiment = state.market_sentiment().await?;
    Ok(Json(MarketSentimentResponse::from(sentiment)))
}

/// API response for one market-sentiment preview.
#[derive(Debug, Serialize)]
struct MarketSentimentResponse {
    /// Bounded Qwen sentiment score in `[-1.0, 1.0]`.
    score: f64,
    /// Stable presentation label derived from the score sign.
    label: MarketSentimentLabel,
}

/// Presentation label for a market-sentiment score.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum MarketSentimentLabel {
    /// Positive Qwen sentiment.
    Positive,
    /// Neutral Qwen sentiment.
    Neutral,
    /// Negative Qwen sentiment.
    Negative,
}

impl From<Sentiment> for MarketSentimentResponse {
    fn from(sentiment: Sentiment) -> Self {
        let score = sentiment.value();
        let label = if score > 0.0 {
            MarketSentimentLabel::Positive
        } else if score < 0.0 {
            MarketSentimentLabel::Negative
        } else {
            MarketSentimentLabel::Neutral
        };

        Self { score, label }
    }
}
