//! Market-sentiment preview HTTP route.

use ai_client::MarketSentimentReport;
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
    Ok(Json(MarketSentimentResponse::from(&sentiment)))
}

/// API response for one market-sentiment preview.
#[derive(Debug, Serialize)]
pub(crate) struct MarketSentimentResponse {
    /// Bounded Qwen sentiment score in `[-1.0, 1.0]`.
    score: f64,
    /// Stable presentation label derived from the score sign.
    label: MarketSentimentLabel,
    /// Concise model explanation grounded in the supplied headlines.
    rationale: String,
    /// Model-supplied uncertainty or risk cautions.
    warnings: Vec<String>,
    /// RSS headlines actually supplied to the model.
    headlines: Vec<MarketSentimentHeadlineResponse>,
}

/// Presentation label for a market-sentiment score.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MarketSentimentLabel {
    /// Positive Qwen sentiment.
    Positive,
    /// Neutral Qwen sentiment.
    Neutral,
    /// Negative Qwen sentiment.
    Negative,
}

/// One source headline included in the market-sentiment response.
#[derive(Debug, Serialize)]
pub(crate) struct MarketSentimentHeadlineResponse {
    /// Original RSS title.
    title: String,
    /// Original RSS HTTP(S) URL when available.
    url: String,
    /// UTC RFC3339 publication timestamp.
    published_at: String,
}

impl From<&MarketSentimentReport> for MarketSentimentResponse {
    fn from(report: &MarketSentimentReport) -> Self {
        let score = report.analysis.sentiment().value();
        let label = if score > 0.0 {
            MarketSentimentLabel::Positive
        } else if score < 0.0 {
            MarketSentimentLabel::Negative
        } else {
            MarketSentimentLabel::Neutral
        };

        Self {
            score,
            label,
            rationale: report.analysis.rationale().to_owned(),
            warnings: report.analysis.warnings().to_vec(),
            headlines: report
                .headlines
                .iter()
                .map(|headline| MarketSentimentHeadlineResponse {
                    title: headline.title.clone(),
                    url: headline.url.clone(),
                    published_at: headline.published_at.to_rfc3339(),
                })
                .collect(),
        }
    }
}
