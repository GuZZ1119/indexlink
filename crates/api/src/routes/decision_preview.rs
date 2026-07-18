//! Decision preview HTTP route.

use std::time::Duration;

use ai_client::Sentiment;
use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        Path, State,
    },
    routing::post,
    Json, Router,
};
use broker::{BrokerOrderAck, BrokerOrderRequest, BrokerOrderSide, BrokerOrderStatus};
use core_domain::{Action, Percentile};
use decision_engine::{
    evaluate_decision, DecisionConfig, DecisionInput, DecisionSentiment, DecisionSignal,
    DecisionWeightMode,
};
use investment_plans::{
    BucketAllocationRatio, ExecutionPreviewStatus, InvestmentPlanExecutionPreview,
    PreviewInvestmentPlanExecution, TwoBucketAllocationConfig,
};
use quant_engine::{FundamentalSignal, TrendRegime, TrendSignal};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use uuid::Uuid;

use crate::{ApiError, ApiState};

const BROKER_SUBMIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Decision preview request DTO.
#[derive(Debug, Deserialize)]
struct DecisionPreviewRequest {
    /// Month day used by the execution preview.
    day_of_month: i16,
    /// Optional bucket allocation used when the plan is due.
    bucket_allocation: Option<TwoBucketAllocationRequest>,
    /// Fundamental signal snapshot.
    fundamental: FundamentalSignalRequest,
    /// Trend signal snapshot.
    trend: TrendSignalRequest,
    /// Optional AI sentiment score; omitted means sentiment unavailable.
    sentiment: Option<SentimentRequest>,
    /// Optional paper order to submit when the decision is executable and due.
    paper_order: Option<PaperOrderRequest>,
}

/// Bucket allocation request DTO.
#[derive(Debug, Deserialize)]
struct TwoBucketAllocationRequest {
    /// Core bucket ratio.
    #[serde(with = "rust_decimal::serde::str")]
    core_ratio: Decimal,
    /// Opportunity bucket ratio.
    #[serde(with = "rust_decimal::serde::str")]
    opportunity_ratio: Decimal,
}

/// Fundamental signal request DTO.
#[derive(Debug, Deserialize)]
struct FundamentalSignalRequest {
    /// Composite fundamental score in `[0.0, 1.0]`.
    score: f64,
    /// Raw CAPE percentile in `[0.0, 1.0]`.
    cape_percentile: f64,
    /// Raw ERP percentile in `[0.0, 1.0]`.
    erp_percentile: f64,
}

/// Trend signal request DTO.
#[derive(Debug, Deserialize)]
struct TrendSignalRequest {
    /// Composite trend score in `[0.0, 1.0]`.
    score: f64,
    /// Raw MA distance percentile in `[0.0, 1.0]`.
    ma_distance_percentile: f64,
    /// Raw RSI percentile in `[0.0, 1.0]`.
    rsi_percentile: f64,
    /// Raw VIX percentile in `[0.0, 1.0]`.
    vix_percentile: f64,
    /// Discrete trend regime.
    regime: TrendRegimeRequest,
}

/// Sentiment request DTO.
#[derive(Debug, Deserialize)]
struct SentimentRequest {
    /// AI sentiment score in `[-1.0, 1.0]`.
    score: f64,
}

/// Optional paper order request DTO.
#[derive(Debug, Deserialize)]
struct PaperOrderRequest {
    /// Stable idempotency key for this preview-triggered paper order.
    idempotency_key: String,
    /// Buy or sell side.
    side: BrokerOrderSideRequest,
    /// Market or limit order type.
    order_type: BrokerOrderTypeRequest,
    /// Positive order quantity.
    #[serde(with = "rust_decimal::serde::str")]
    quantity: Decimal,
    /// Positive limit price when `order_type` is limit.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    limit_price: Option<Decimal>,
}

/// API trend regime values.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TrendRegimeRequest {
    /// Overheated market regime.
    Overheated,
    /// Neutral market regime.
    Neutral,
    /// Falling-knife market regime.
    FallingKnife,
}

/// API broker order side values.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BrokerOrderSideRequest {
    /// Buy side.
    Buy,
    /// Sell side.
    Sell,
}

/// API broker order type values.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BrokerOrderTypeRequest {
    /// Market order.
    Market,
    /// Limit order.
    Limit,
}

/// Decision preview response DTO.
#[derive(Debug, Serialize)]
struct DecisionPreviewResponse {
    /// Execution preview from the investment-plan service.
    execution: InvestmentPlanExecutionPreview,
    /// Decision result safe for API clients.
    decision: DecisionResponse,
    /// Paper order acknowledgement when an executable due preview submitted an order.
    #[serde(skip_serializing_if = "Option::is_none")]
    paper_order_ack: Option<BrokerOrderAck>,
    /// Human-readable summary for demo UI.
    summary: String,
}

/// API-facing decision response.
#[derive(Debug, Serialize)]
struct DecisionResponse {
    /// Final investability score.
    final_score: f64,
    /// Contribution multiplier.
    multiplier: f64,
    /// Final action label.
    action: ActionResponse,
    /// Weight mode used by the decision engine.
    weight_mode: DecisionWeightModeResponse,
    /// Fundamental contribution score after direction normalization.
    fundamental_score: f64,
    /// Trend timing contribution score after safety normalization.
    trend_score: f64,
    /// Sentiment contribution score when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    sentiment_score: Option<f64>,
}

/// API action values.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ActionResponse {
    /// Increase contribution.
    Overweight,
    /// Standard contribution.
    Standard,
    /// Delay execution tactically.
    TacticalDelay,
    /// Reduce contribution.
    Underweight,
    /// Skip this execution.
    Skip,
}

/// API decision weight mode values.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum DecisionWeightModeResponse {
    /// Normal 70/20/10 weights.
    Normal,
    /// Sentiment-unavailable fallback weights.
    SentimentUnavailable,
}

/// Build decision preview routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new().route(
        "/investment-plans/:id/decision-preview",
        post(preview_decision),
    )
}

/// Preview one investment decision and optionally submit a configured paper order.
async fn preview_decision(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<DecisionPreviewRequest>, JsonRejection>,
) -> Result<Json<DecisionPreviewResponse>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;

    let execution_input = PreviewInvestmentPlanExecution::new(input.day_of_month)?;
    let decision_input = input.clone_decision_input()?;
    let bucket_config = input
        .bucket_allocation
        .map(TwoBucketAllocationRequest::into_domain)
        .transpose()?;

    let execution = match bucket_config {
        Some(bucket_config) => {
            state
                .plans()
                .preview_execution_with_buckets(id, execution_input, bucket_config)
                .await?
        }
        None => state.plans().preview_execution(id, execution_input).await?,
    };
    let decision = evaluate_decision(&decision_input, &DecisionConfig::default());
    let paper_order_ack =
        maybe_submit_paper_order(&state, &execution, &decision, input.paper_order).await?;
    let summary = summarize_decision(&execution, &decision, paper_order_ack.as_ref());

    Ok(Json(DecisionPreviewResponse {
        execution,
        decision: DecisionResponse::from_signal(&decision),
        paper_order_ack,
        summary,
    }))
}

impl DecisionPreviewRequest {
    fn clone_decision_input(&self) -> Result<DecisionInput, ApiError> {
        Ok(DecisionInput {
            fundamental: self.fundamental.clone_signal()?,
            trend: self.trend.clone_signal()?,
            sentiment: self
                .sentiment
                .as_ref()
                .map(SentimentRequest::to_domain)
                .transpose()?
                .map_or(DecisionSentiment::Unavailable, DecisionSentiment::Available),
        })
    }
}

impl TwoBucketAllocationRequest {
    fn into_domain(self) -> Result<TwoBucketAllocationConfig, ApiError> {
        TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(self.core_ratio)?,
            BucketAllocationRatio::new(self.opportunity_ratio)?,
        )
        .map_err(|_| ApiError::BadRequest)
    }
}

impl FundamentalSignalRequest {
    fn clone_signal(&self) -> Result<FundamentalSignal, ApiError> {
        Ok(FundamentalSignal {
            score: percentile(self.score)?,
            cape_percentile: percentile(self.cape_percentile)?,
            erp_percentile: percentile(self.erp_percentile)?,
        })
    }
}

impl TrendSignalRequest {
    fn clone_signal(&self) -> Result<TrendSignal, ApiError> {
        Ok(TrendSignal {
            score: percentile(self.score)?,
            ma_distance_percentile: percentile(self.ma_distance_percentile)?,
            rsi_percentile: percentile(self.rsi_percentile)?,
            vix_percentile: percentile(self.vix_percentile)?,
            regime: self.regime.to_domain(),
        })
    }
}

impl SentimentRequest {
    fn to_domain(&self) -> Result<Sentiment, ApiError> {
        Sentiment::new(self.score).ok_or(ApiError::BadRequest)
    }
}

impl PaperOrderRequest {
    fn into_domain(self, symbol: &str) -> Result<BrokerOrderRequest, ApiError> {
        match self.order_type {
            BrokerOrderTypeRequest::Market => {
                if self.limit_price.is_some() {
                    return Err(ApiError::BadRequest);
                }
                BrokerOrderRequest::market(
                    self.idempotency_key,
                    symbol,
                    self.side.into(),
                    self.quantity,
                    broker::BrokerEnvironment::Paper,
                )
            }
            BrokerOrderTypeRequest::Limit => BrokerOrderRequest::limit(
                self.idempotency_key,
                symbol,
                self.side.into(),
                self.quantity,
                self.limit_price.ok_or(ApiError::BadRequest)?,
                broker::BrokerEnvironment::Paper,
            ),
        }
        .map_err(|_| ApiError::BadRequest)
    }
}

impl TrendRegimeRequest {
    fn to_domain(&self) -> TrendRegime {
        match self {
            Self::Overheated => TrendRegime::Overheated,
            Self::Neutral => TrendRegime::Neutral,
            Self::FallingKnife => TrendRegime::FallingKnife,
        }
    }
}

impl From<BrokerOrderSideRequest> for BrokerOrderSide {
    fn from(value: BrokerOrderSideRequest) -> Self {
        match value {
            BrokerOrderSideRequest::Buy => Self::Buy,
            BrokerOrderSideRequest::Sell => Self::Sell,
        }
    }
}

impl DecisionResponse {
    fn from_signal(signal: &DecisionSignal) -> Self {
        Self {
            final_score: signal.final_score.value(),
            multiplier: signal.multiplier.value(),
            action: signal.action.into(),
            weight_mode: signal.weight_mode.into(),
            fundamental_score: signal.fundamental_score.value(),
            trend_score: signal.trend_score.value(),
            sentiment_score: signal.sentiment_score.map(Percentile::value),
        }
    }
}

impl From<Action> for ActionResponse {
    fn from(value: Action) -> Self {
        match value {
            Action::Overweight => Self::Overweight,
            Action::Standard => Self::Standard,
            Action::TacticalDelay => Self::TacticalDelay,
            Action::Underweight => Self::Underweight,
            Action::Skip => Self::Skip,
        }
    }
}

impl From<DecisionWeightMode> for DecisionWeightModeResponse {
    fn from(value: DecisionWeightMode) -> Self {
        match value {
            DecisionWeightMode::Normal => Self::Normal,
            DecisionWeightMode::SentimentUnavailable => Self::SentimentUnavailable,
        }
    }
}

fn percentile(value: f64) -> Result<Percentile, ApiError> {
    Percentile::new(value).ok_or(ApiError::BadRequest)
}

async fn maybe_submit_paper_order(
    state: &ApiState,
    execution: &InvestmentPlanExecutionPreview,
    decision: &DecisionSignal,
    paper_order: Option<PaperOrderRequest>,
) -> Result<Option<BrokerOrderAck>, ApiError> {
    let request = paper_order
        .map(|order| order.into_domain(&execution.symbol))
        .transpose()?;

    if execution.status != ExecutionPreviewStatus::Due
        || matches!(decision.action, Action::Skip | Action::TacticalDelay)
    {
        return Ok(None);
    }

    let Some(request) = request else {
        return Ok(None);
    };

    timeout(BROKER_SUBMIT_TIMEOUT, state.broker().submit_order(request))
        .await
        .map_err(|_| ApiError::ServiceUnavailable)?
        .map(Some)
        .map_err(Into::into)
}

fn summarize_decision(
    execution: &InvestmentPlanExecutionPreview,
    decision: &DecisionSignal,
    ack: Option<&BrokerOrderAck>,
) -> String {
    let action = match decision.action {
        Action::Overweight => "overweight",
        Action::Standard => "standard",
        Action::TacticalDelay => "tactical delay",
        Action::Underweight => "underweight",
        Action::Skip => "skip",
    };
    let order = match ack.map(BrokerOrderAck::status) {
        Some(BrokerOrderStatus::Accepted) => "paper order accepted",
        Some(BrokerOrderStatus::Duplicate) => "paper order deduplicated",
        None => "no paper order submitted",
    };

    format!(
        "Decision preview for {} is {:?}: action={}, multiplier={:.2}, {}.",
        execution.symbol,
        execution.status,
        action,
        decision.multiplier.value(),
        order
    )
}
