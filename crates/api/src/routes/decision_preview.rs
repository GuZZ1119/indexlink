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
use decision_records::{CompleteDecisionRecord, CreateDecisionRecord, DecisionExecutionStatus};
use investment_plans::{
    BucketAllocationRatio, ExecutionPreviewStatus, InvestmentPlanExecutionPreview,
    PreviewInvestmentPlanExecution, TwoBucketAllocationConfig,
};
use quant_engine::{FundamentalSignal, TrendRegime, TrendSignal};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::timeout;
use uuid::Uuid;

use crate::{ApiError, ApiState};

const BROKER_SUBMIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Decision preview request DTO.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DecisionPreviewRequest {
    /// Month day used by the execution preview.
    day_of_month: i16,
    /// Optional bucket allocation used when the plan is due.
    bucket_allocation: Option<TwoBucketAllocationRequest>,
    /// Fundamental signal snapshot.
    fundamental: FundamentalSignalRequest,
    /// Trend signal snapshot.
    trend: TrendSignalRequest,
    /// Optional paper order to submit when the decision is executable and due.
    paper_order: Option<PaperOrderRequest>,
}

/// Bucket allocation request DTO.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TwoBucketAllocationRequest {
    /// Core bucket ratio.
    #[serde(with = "rust_decimal::serde::str")]
    core_ratio: Decimal,
    /// Opportunity bucket ratio.
    #[serde(with = "rust_decimal::serde::str")]
    opportunity_ratio: Decimal,
}

/// Fundamental signal request DTO.
#[derive(Debug, Deserialize, Serialize)]
struct FundamentalSignalRequest {
    /// Composite fundamental score in `[0.0, 1.0]`.
    score: f64,
    /// Raw CAPE percentile in `[0.0, 1.0]`.
    cape_percentile: f64,
    /// Raw ERP percentile in `[0.0, 1.0]`.
    erp_percentile: f64,
}

/// Trend signal request DTO.
#[derive(Debug, Deserialize, Serialize)]
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

/// Optional paper order request DTO.
#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum BrokerOrderSideRequest {
    /// Buy side.
    Buy,
    /// Sell side.
    Sell,
}

/// API broker order type values.
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    let fundamental = input.fundamental.clone_signal()?;
    let trend = input.trend.clone_signal()?;
    let bucket_config = input
        .bucket_allocation
        .clone()
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
    let paper_order = input
        .paper_order
        .clone()
        .map(|order| order.into_domain(&execution.symbol))
        .transpose()?;
    let market_sentiment = market_sentiment_for_decision(&state).await;
    let decision_input = DecisionInput {
        fundamental,
        trend,
        sentiment: market_sentiment
            .map_or(DecisionSentiment::Unavailable, DecisionSentiment::Available),
    };
    let decision = evaluate_decision(&decision_input, &DecisionConfig::default());
    let decision_response = DecisionResponse::from_signal(&decision);
    let should_submit = should_submit_paper_order(&execution, &decision, paper_order.as_ref());
    let preliminary_summary = summarize_decision(&execution, &decision, None);
    let persisted = state
        .decision_records()
        .create(record_input(
            id,
            &input,
            &execution,
            &decision,
            paper_order.as_ref(),
            None,
            preliminary_summary,
        )?)
        .await?;
    let paper_order_ack = if should_submit {
        let request = paper_order.as_ref().ok_or(ApiError::ServiceUnavailable)?;
        let ack = submit_paper_order(&state, request).await?;
        let summary = summarize_decision(&execution, &decision, Some(&ack));
        if let Err(error) = state
            .decision_records()
            .complete_broker_order(
                persisted.id,
                CompleteDecisionRecord {
                    broker_order_ack: snapshot(&ack)?,
                    summary: summary.clone(),
                },
            )
            .await
        {
            tracing::error!(error = %error, record_id = %persisted.id, "paper order accepted but decision record completion failed");
        }
        Some(ack)
    } else {
        None
    };
    let summary = summarize_decision(&execution, &decision, paper_order_ack.as_ref());

    Ok(Json(DecisionPreviewResponse {
        execution,
        decision: decision_response,
        paper_order_ack,
        summary,
    }))
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

/// Return whether the validated order is safe and eligible to submit.
fn should_submit_paper_order(
    execution: &InvestmentPlanExecutionPreview,
    decision: &DecisionSignal,
    paper_order: Option<&BrokerOrderRequest>,
) -> bool {
    paper_order.is_some()
        && execution.status == ExecutionPreviewStatus::Due
        && !matches!(decision.action, Action::Skip | Action::TacticalDelay)
}

/// Submit one already-validated paper order through the configured broker.
async fn submit_paper_order(
    state: &ApiState,
    request: &BrokerOrderRequest,
) -> Result<BrokerOrderAck, ApiError> {
    timeout(
        BROKER_SUBMIT_TIMEOUT,
        state.broker().submit_order(request.clone()),
    )
    .await
    .map_err(|_| ApiError::ServiceUnavailable)?
    .map_err(Into::into)
}

/// Fetch Qwen market sentiment and safely fall back to the engine's 90/10/0 mode.
async fn market_sentiment_for_decision(state: &ApiState) -> Option<Sentiment> {
    match state.market_sentiment().await {
        Ok(sentiment) => Some(sentiment),
        Err(ApiError::ServiceUnavailable) => {
            tracing::warn!("market sentiment unavailable; decision preview uses fallback weights");
            None
        }
        Err(error) => {
            tracing::error!(error = %error, "unexpected market sentiment error; decision preview uses fallback weights");
            None
        }
    }
}

/// Build a complete local audit snapshot before any optional broker side effect.
fn record_input(
    plan_id: Uuid,
    input: &DecisionPreviewRequest,
    execution: &InvestmentPlanExecutionPreview,
    decision: &DecisionSignal,
    paper_order: Option<&BrokerOrderRequest>,
    paper_order_ack: Option<&BrokerOrderAck>,
    summary: String,
) -> Result<CreateDecisionRecord, ApiError> {
    Ok(CreateDecisionRecord {
        plan_id,
        symbol: execution.symbol.clone(),
        currency: execution.currency.clone(),
        execution_status: execution_status(execution.status),
        planned_contribution: execution
            .planned_contribution
            .map(|value| value.to_string()),
        execution_snapshot: snapshot(execution)?,
        fundamental_snapshot: snapshot(&input.fundamental)?,
        trend_snapshot: snapshot(&input.trend)?,
        sentiment_snapshot: decision_market_sentiment_snapshot(decision),
        decision_snapshot: decision_snapshot(decision),
        broker_order_request: paper_order.map(snapshot).transpose()?,
        broker_order_ack: paper_order_ack.map(snapshot).transpose()?,
        summary,
    })
}

/// Return a stable audit snapshot for an automatically retrieved market sentiment.
fn market_sentiment_snapshot(sentiment: Sentiment) -> Value {
    json!({"source": "market_sentiment", "score": sentiment.value()})
}

/// Return the automatic sentiment input snapshot retained by the decision engine.
fn decision_market_sentiment_snapshot(decision: &DecisionSignal) -> Option<Value> {
    match decision.input.sentiment {
        DecisionSentiment::Available(sentiment) => Some(market_sentiment_snapshot(sentiment)),
        DecisionSentiment::Unavailable => None,
    }
}

/// Convert an execution preview status into its persisted audit representation.
fn execution_status(status: ExecutionPreviewStatus) -> DecisionExecutionStatus {
    match status {
        ExecutionPreviewStatus::Due => DecisionExecutionStatus::Due,
        ExecutionPreviewStatus::Waiting => DecisionExecutionStatus::Waiting,
        ExecutionPreviewStatus::Inactive => DecisionExecutionStatus::Inactive,
    }
}

/// Serialize one trusted in-process value into a JSON audit snapshot.
fn snapshot(value: &impl Serialize) -> Result<Value, ApiError> {
    serde_json::to_value(value).map_err(|error| {
        tracing::error!(error = %error, "decision preview audit snapshot serialization failed");
        ApiError::ServiceUnavailable
    })
}

/// Build the decision-output snapshot, including the effective weights.
fn decision_snapshot(decision: &DecisionSignal) -> Value {
    json!({
        "final_score": decision.final_score.value(),
        "multiplier": decision.multiplier.value(),
        "action": action_label(decision.action),
        "weight_mode": weight_mode_label(decision.weight_mode),
        "weights": {
            "fundamental_weight": decision.weights.fundamental_weight.value(),
            "trend_weight": decision.weights.trend_weight.value(),
            "sentiment_weight": decision.weights.sentiment_weight.value(),
        },
        "fundamental_score": decision.fundamental_score.value(),
        "trend_score": decision.trend_score.value(),
        "sentiment_score": decision.sentiment_score.map(Percentile::value),
    })
}

/// Return the stable persisted label for a decision action.
fn action_label(action: Action) -> &'static str {
    match action {
        Action::Overweight => "overweight",
        Action::Standard => "standard",
        Action::TacticalDelay => "tactical_delay",
        Action::Underweight => "underweight",
        Action::Skip => "skip",
    }
}

/// Return the stable persisted label for the selected decision-weight mode.
fn weight_mode_label(mode: DecisionWeightMode) -> &'static str {
    match mode {
        DecisionWeightMode::Normal => "normal",
        DecisionWeightMode::SentimentUnavailable => "sentiment_unavailable",
    }
}

fn summarize_decision(
    execution: &InvestmentPlanExecutionPreview,
    decision: &DecisionSignal,
    ack: Option<&BrokerOrderAck>,
) -> String {
    let execution_status = match execution.status {
        ExecutionPreviewStatus::Due => "due",
        ExecutionPreviewStatus::Waiting => "waiting",
        ExecutionPreviewStatus::Inactive => "inactive",
    };
    let contribution = execution
        .planned_contribution
        .map_or_else(|| "none".to_owned(), |value| value.to_string());
    let fundamental = score_interpretation(decision.fundamental_score.value());
    let trend = score_interpretation(decision.trend_score.value());
    let sentiment = decision.sentiment_score.map_or_else(
        || "unavailable".to_owned(),
        |value| format!("{:.2}", value.value()),
    );
    let bucket_split = execution.bucket_split.map_or_else(
        || "none".to_owned(),
        |split| {
            format!(
                "core={} {}, opportunity={} {}",
                split.core_contribution(),
                execution.currency,
                split.opportunity_contribution(),
                execution.currency,
            )
        },
    );
    let order = match ack.map(BrokerOrderAck::status) {
        Some(BrokerOrderStatus::Accepted) => "paper order accepted",
        Some(BrokerOrderStatus::Duplicate) => "paper order deduplicated",
        None => "no paper order submitted",
    };

    format!(
        "Decision preview for {}: execution={}; planned_contribution={} {}; fundamental_investability={:.2} ({}); trend_timing={:.2} ({}, regime={:?}); market_sentiment={}; weight_mode={}; final_score={:.2}; multiplier={:.2}; action={}; bucket_split={}; {}.",
        execution.symbol,
        execution_status,
        contribution,
        execution.currency,
        decision.fundamental_score.value(),
        fundamental,
        decision.trend_score.value(),
        trend,
        decision.input.trend.regime,
        sentiment,
        weight_mode_label(decision.weight_mode),
        decision.final_score.value(),
        decision.multiplier.value(),
        action_label(decision.action),
        bucket_split,
        order
    )
}

/// Return a stable, intentionally coarse explanation for a normalized score.
fn score_interpretation(score: f64) -> &'static str {
    if score <= 0.33 {
        "cautious"
    } else if score >= 0.67 {
        "supportive"
    } else {
        "neutral"
    }
}
