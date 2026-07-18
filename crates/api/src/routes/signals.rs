//! Fundamental and trend signal preview HTTP routes.

use axum::{extract::rejection::JsonRejection, routing::post, Json, Router};
use quant_engine::{
    evaluate_fundamental, evaluate_trend, FundamentalConfig, FundamentalSnapshot, TrendConfig,
    TrendRegime, TrendSignal, TrendSnapshot,
};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiState};

/// Build quant-signal preview routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/signals/fundamental/preview", post(preview_fundamental))
        .route("/signals/trend/preview", post(preview_trend))
}

/// Calculate one 70% fundamental signal from caller-supplied historical snapshots.
async fn preview_fundamental(
    input: Result<Json<FundamentalSignalRequest>, JsonRejection>,
) -> Result<Json<FundamentalSignalResponse>, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let signal = evaluate_fundamental(
        &FundamentalSnapshot {
            cape_history: input.cape_history,
            cape_current: input.cape_current,
            erp_history: input.erp_history,
            erp_current: input.erp_current,
        },
        &FundamentalConfig::default(),
    )
    .map_err(|_| ApiError::BadRequest)?;

    Ok(Json(FundamentalSignalResponse {
        score: signal.score.value(),
        cape_percentile: signal.cape_percentile.value(),
        erp_percentile: signal.erp_percentile.value(),
    }))
}

/// Calculate one 20% trend signal from caller-supplied historical snapshots.
async fn preview_trend(
    input: Result<Json<TrendSignalRequest>, JsonRejection>,
) -> Result<Json<TrendSignalResponse>, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let signal = evaluate_trend(
        &TrendSnapshot {
            ma_distance_history: input.ma_distance_history,
            ma_distance_current: input.ma_distance_current,
            rsi_history: input.rsi_history,
            rsi_current: input.rsi_current,
            vix_history: input.vix_history,
            vix_current: input.vix_current,
        },
        &TrendConfig::default(),
    )
    .map_err(|_| ApiError::BadRequest)?;

    Ok(Json(TrendSignalResponse::from(signal)))
}

/// Request DTO for a 70% fundamental signal preview.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FundamentalSignalRequest {
    /// Historical monthly Shiller CAPE values, oldest first.
    cape_history: Vec<f64>,
    /// Current Shiller CAPE value.
    cape_current: f64,
    /// Historical monthly equity-risk-premium values, oldest first.
    erp_history: Vec<f64>,
    /// Current equity-risk-premium value.
    erp_current: f64,
}

/// Request DTO for a 20% trend signal preview.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrendSignalRequest {
    /// Historical monthly MA200-distance values, oldest first.
    ma_distance_history: Vec<f64>,
    /// Current MA200-distance value.
    ma_distance_current: f64,
    /// Historical monthly RSI values, oldest first.
    rsi_history: Vec<f64>,
    /// Current RSI value.
    rsi_current: f64,
    /// Historical monthly VIX values, oldest first.
    vix_history: Vec<f64>,
    /// Current VIX value.
    vix_current: f64,
}

/// Response returned by the fundamental signal preview.
#[derive(Debug, Serialize)]
struct FundamentalSignalResponse {
    /// Composite valuation-position score where lower values are historically cheaper.
    score: f64,
    /// Raw CAPE percentile retained for audit.
    cape_percentile: f64,
    /// Raw ERP percentile retained for audit.
    erp_percentile: f64,
}

/// Response returned by the trend signal preview.
#[derive(Debug, Serialize)]
struct TrendSignalResponse {
    /// Composite trend score before Decision Engine timing normalization.
    score: f64,
    /// Raw MA200-distance percentile retained for audit.
    ma_distance_percentile: f64,
    /// Raw RSI percentile retained for audit.
    rsi_percentile: f64,
    /// Raw VIX percentile retained for audit.
    vix_percentile: f64,
    /// Discrete trend timing regime.
    regime: TrendRegimeResponse,
}

/// API representation of a trend timing regime.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum TrendRegimeResponse {
    /// Strong-uptrend or overbought timing risk.
    Overheated,
    /// Neither overheated nor falling-knife timing risk applies.
    Neutral,
    /// High-volatility falling-knife timing risk.
    FallingKnife,
}

impl From<TrendSignal> for TrendSignalResponse {
    fn from(signal: TrendSignal) -> Self {
        Self {
            score: signal.score.value(),
            ma_distance_percentile: signal.ma_distance_percentile.value(),
            rsi_percentile: signal.rsi_percentile.value(),
            vix_percentile: signal.vix_percentile.value(),
            regime: signal.regime.into(),
        }
    }
}

impl From<TrendRegime> for TrendRegimeResponse {
    fn from(regime: TrendRegime) -> Self {
        match regime {
            TrendRegime::Overheated => Self::Overheated,
            TrendRegime::Neutral => Self::Neutral,
            TrendRegime::FallingKnife => Self::FallingKnife,
        }
    }
}
