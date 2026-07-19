//! Automatic market-signal input route.

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use market_data::MarketSignalInput;
use serde::Serialize;

use crate::{ApiError, ApiState};

/// Build automatic market-signal input routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new().route("/signals/market-input/:symbol", get(fetch_market_input))
}

/// Fetch a source-labelled automatic signal input for one selected plan symbol.
async fn fetch_market_input(
    State(state): State<ApiState>,
    Path(symbol): Path<String>,
) -> Result<Json<MarketSignalInputResponse>, ApiError> {
    Ok(Json(MarketSignalInputResponse::from(
        state.market_signal_input(&symbol).await?,
    )))
}

/// Safe automatic market-signal response consumed by the local dashboard.
#[derive(Debug, Serialize)]
struct MarketSignalInputResponse {
    /// Normalized US security symbol.
    symbol: String,
    /// Latest OpenD trading date included in the technical snapshot.
    as_of: String,
    /// Fundamental inputs calculated from public monthly CAPE and Treasury data.
    fundamental: FundamentalInputResponse,
    /// Technical inputs calculated from OpenD daily closes and public VIX data.
    trend: TrendInputResponse,
    /// Provider and calculation notes shown to the operator before a decision.
    sources: MarketDataSourcesResponse,
}

/// Fundamental snapshot fields.
#[derive(Debug, Serialize)]
struct FundamentalInputResponse {
    /// Monthly Shiller CAPE values, oldest first.
    cape_history: Vec<f64>,
    /// Latest Shiller CAPE value.
    cape_current: f64,
    /// Monthly ERP proxy values, oldest first.
    erp_history: Vec<f64>,
    /// Latest ERP proxy value.
    erp_current: f64,
}

/// Trend snapshot fields.
#[derive(Debug, Serialize)]
struct TrendInputResponse {
    /// Monthly MA200 distances, oldest first.
    ma_distance_history: Vec<f64>,
    /// Latest MA200 distance.
    ma_distance_current: f64,
    /// Monthly 14-day RSI values, oldest first.
    rsi_history: Vec<f64>,
    /// Latest RSI value.
    rsi_current: f64,
    /// Monthly VIX values, oldest first.
    vix_history: Vec<f64>,
    /// Latest VIX value.
    vix_current: f64,
}

/// Human-readable source disclosures for the automatic snapshot.
#[derive(Debug, Serialize)]
struct MarketDataSourcesResponse {
    /// Price and technical-indicator source.
    price: &'static str,
    /// Valuation source and ERP-proxy formula.
    fundamental: &'static str,
    /// Volatility source.
    volatility: &'static str,
}

impl From<MarketSignalInput> for MarketSignalInputResponse {
    fn from(input: MarketSignalInput) -> Self {
        Self {
            symbol: input.symbol,
            as_of: input.as_of,
            fundamental: FundamentalInputResponse {
                cape_history: input.cape_history,
                cape_current: input.cape_current,
                erp_history: input.erp_history,
                erp_current: input.erp_current,
            },
            trend: TrendInputResponse {
                ma_distance_history: input.ma_distance_history,
                ma_distance_current: input.ma_distance_current,
                rsi_history: input.rsi_history,
                rsi_current: input.rsi_current,
                vix_history: input.vix_history,
                vix_current: input.vix_current,
            },
            sources: MarketDataSourcesResponse {
                price: "local OpenD daily close; MA200 and RSI are computed locally",
                fundamental:
                    "Shiller CAPE monthly table; ERP proxy = 100 / CAPE - US Treasury 10-year yield",
                volatility: "Cboe VIX monthly last observation",
            },
        }
    }
}
