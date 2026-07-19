//! Decision preview HTTP route.

use std::time::Duration;

use ai_client::MarketSentimentReport;
use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        Path, State,
    },
    routing::post,
    Json, Router,
};
use broker::{BrokerOrderAck, BrokerOrderRequest, BrokerOrderSide, BrokerOrderStatus};
use chrono::{Datelike, Utc};
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
use market_data::MarketSignalInput;
use quant_engine::{
    evaluate_fundamental, evaluate_trend, FundamentalConfig, FundamentalSignal,
    FundamentalSnapshot, TrendConfig, TrendRegime, TrendSignal, TrendSnapshot,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::timeout;
use uuid::Uuid;

use crate::{ApiError, ApiState};

use super::market_sentiment::MarketSentimentResponse;

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
    /// Trusted source disclosure attached only by server-side automatic orchestration.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    input_source: Option<Value>,
}

/// Server-sourced decision request that deliberately excludes 70/20 signal fields.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AutomaticDecisionPreviewRequest {
    /// Optional bucket allocation used when the plan is due.
    bucket_allocation: Option<TwoBucketAllocationRequest>,
    /// Optional operator-confirmed paper order submitted only after automatic evaluation.
    paper_order: Option<PaperOrderRequest>,
}

/// Origin of an audit record's 70/20 inputs.
#[derive(Debug, Clone, Copy)]
enum DecisionTrigger {
    /// An operator supplied validated signal values through the legacy preview endpoint.
    ManualInput,
    /// An operator explicitly requested server-sourced automatic inputs.
    AutomaticPreview,
    /// The fixed-monthly background scheduler created the automatic decision.
    AutomaticScheduler,
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
    /// Persisted local audit-record ID for this decision.
    audit_record_id: Uuid,
    /// Execution preview from the investment-plan service.
    execution: InvestmentPlanExecutionPreview,
    /// Decision result safe for API clients.
    decision: DecisionResponse,
    /// AI rationale, risk warnings, and RSS sources used for this decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    market_sentiment: Option<MarketSentimentResponse>,
    /// Paper order acknowledgement when an executable due preview submitted an order.
    #[serde(skip_serializing_if = "Option::is_none")]
    paper_order_ack: Option<BrokerOrderAck>,
    /// Human-readable summary for demo UI.
    summary: String,
}

/// Result counters emitted by one fixed-monthly automatic scheduler tick.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScheduledDecisionRunSummary {
    /// Due active plans for which a new automatic audit record was created.
    pub created: u32,
    /// Due plans already claimed for the same UTC calendar day.
    pub already_claimed: u32,
    /// Due plans skipped because automatic market data was unavailable or invalid.
    pub unavailable: u32,
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
    Router::new()
        .route(
            "/investment-plans/:id/decision-preview",
            post(preview_decision),
        )
        .route(
            "/investment-plans/:id/automatic-decision-preview",
            post(preview_automatic_decision),
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

    Ok(Json(
        preview_decision_input(&state, id, input, DecisionTrigger::ManualInput).await?,
    ))
}

/// Run one server-sourced decision preview without accepting caller-supplied 70/20 inputs.
async fn preview_automatic_decision(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<AutomaticDecisionPreviewRequest>, JsonRejection>,
) -> Result<Json<DecisionPreviewResponse>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let day_of_month = i16::try_from(Utc::now().day()).map_err(|_| ApiError::ServiceUnavailable)?;
    Ok(Json(
        preview_automatic_for_plan(
            &state,
            id,
            day_of_month,
            DecisionTrigger::AutomaticPreview,
            input,
        )
        .await?,
    ))
}

/// Execute a due-plan scheduler tick using the UTC calendar day as its fixed-monthly trigger.
pub(crate) async fn run_due_decisions(
    state: &ApiState,
) -> Result<ScheduledDecisionRunSummary, ApiError> {
    let now = Utc::now();
    let day_of_month = i16::try_from(now.day()).map_err(|_| ApiError::ServiceUnavailable)?;
    let scheduled_for = now.date_naive().to_string();
    let mut summary = ScheduledDecisionRunSummary::default();

    for plan in state.plans().list().await? {
        if !plan.is_active || plan.schedule_day != day_of_month {
            continue;
        }
        match preview_automatic_for_plan_with_claim(state, plan.id, day_of_month, &scheduled_for)
            .await
        {
            Ok(Some(_)) => summary.created += 1,
            Ok(None) => summary.already_claimed += 1,
            Err(ApiError::ServiceUnavailable | ApiError::BadRequest) => {
                summary.unavailable += 1;
                tracing::warn!(plan_id = %plan.id, "automatic decision skipped because market inputs were unavailable");
            }
            Err(error) => return Err(error),
        }
    }

    Ok(summary)
}

/// Build source-labelled automatic inputs, claim the plan/day key, then persist one audit record.
async fn preview_automatic_for_plan_with_claim(
    state: &ApiState,
    plan_id: Uuid,
    day_of_month: i16,
    scheduled_for: &str,
) -> Result<Option<DecisionPreviewResponse>, ApiError> {
    let plan = state.plans().get(plan_id).await?;
    let input = automatic_decision_input(
        state,
        &plan.symbol,
        day_of_month,
        AutomaticDecisionPreviewRequest {
            bucket_allocation: None,
            paper_order: None,
        },
    )
    .await?;
    if !state
        .claim_scheduled_decision(plan_id, scheduled_for)
        .await?
    {
        return Ok(None);
    }
    preview_decision_input(state, plan_id, input, DecisionTrigger::AutomaticScheduler)
        .await
        .map(Some)
}

/// Build one automatic preview from trusted provider data for a selected investment symbol.
async fn preview_automatic_for_plan(
    state: &ApiState,
    plan_id: Uuid,
    day_of_month: i16,
    trigger: DecisionTrigger,
    options: AutomaticDecisionPreviewRequest,
) -> Result<DecisionPreviewResponse, ApiError> {
    let plan = state.plans().get(plan_id).await?;
    let input = automatic_decision_input(state, &plan.symbol, day_of_month, options).await?;
    preview_decision_input(state, plan_id, input, trigger).await
}

/// Resolve automatic market snapshots into the same validated request shape used by the engine.
async fn automatic_decision_input(
    state: &ApiState,
    symbol: &str,
    day_of_month: i16,
    options: AutomaticDecisionPreviewRequest,
) -> Result<DecisionPreviewRequest, ApiError> {
    let input = state.market_signal_input(symbol).await?;
    let fundamental = evaluate_fundamental(
        &FundamentalSnapshot {
            cape_history: input.cape_history.clone(),
            cape_current: input.cape_current,
            erp_history: input.erp_history.clone(),
            erp_current: input.erp_current,
        },
        &FundamentalConfig::default(),
    )
    .map_err(|_| ApiError::ServiceUnavailable)?;
    let trend = evaluate_trend(
        &TrendSnapshot {
            ma_distance_history: input.ma_distance_history.clone(),
            ma_distance_current: input.ma_distance_current,
            rsi_history: input.rsi_history.clone(),
            rsi_current: input.rsi_current,
            vix_history: input.vix_history.clone(),
            vix_current: input.vix_current,
        },
        &TrendConfig::default(),
    )
    .map_err(|_| ApiError::ServiceUnavailable)?;

    Ok(DecisionPreviewRequest {
        day_of_month,
        bucket_allocation: options.bucket_allocation,
        fundamental: FundamentalSignalRequest::from(fundamental),
        trend: TrendSignalRequest::from(trend),
        paper_order: options.paper_order,
        input_source: Some(automatic_source_snapshot(&input)),
    })
}

/// Execute a resolved decision and create its audit record before any optional broker side effect.
async fn preview_decision_input(
    state: &ApiState,
    id: Uuid,
    input: DecisionPreviewRequest,
    trigger: DecisionTrigger,
) -> Result<DecisionPreviewResponse, ApiError> {
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
    let market_sentiment = market_sentiment_for_decision(state).await;
    let market_sentiment_response = market_sentiment.as_ref().map(MarketSentimentResponse::from);
    let decision_input = DecisionInput {
        fundamental,
        trend,
        sentiment: market_sentiment
            .as_ref()
            .map_or(DecisionSentiment::Unavailable, |report| {
                DecisionSentiment::Available(report.analysis.sentiment())
            }),
    };
    let decision = evaluate_decision(&decision_input, &DecisionConfig::default());
    let decision_response = DecisionResponse::from_signal(&decision);
    let should_submit = should_submit_paper_order(&execution, &decision, paper_order.as_ref());
    let preliminary_summary = summarize_decision(&execution, &decision, None);
    let persisted = state
        .decision_records()
        .create(record_input(DecisionRecordContext {
            plan_id: id,
            input: &input,
            execution: &execution,
            decision: &decision,
            market_sentiment: market_sentiment.as_ref(),
            trigger,
            paper_order: paper_order.as_ref(),
            paper_order_ack: None,
            summary: preliminary_summary,
        })?)
        .await?;
    let paper_order_ack = if should_submit {
        let request = paper_order.as_ref().ok_or(ApiError::ServiceUnavailable)?;
        let ack = submit_paper_order(state, request).await?;
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
        state.record_accepted_paper_order(id, &ack, request).await;
        Some(ack)
    } else {
        None
    };
    let summary = summarize_decision(&execution, &decision, paper_order_ack.as_ref());

    Ok(DecisionPreviewResponse {
        audit_record_id: persisted.id,
        execution,
        decision: decision_response,
        market_sentiment: market_sentiment_response,
        paper_order_ack,
        summary,
    })
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

impl From<FundamentalSignal> for FundamentalSignalRequest {
    fn from(signal: FundamentalSignal) -> Self {
        Self {
            score: signal.score.value(),
            cape_percentile: signal.cape_percentile.value(),
            erp_percentile: signal.erp_percentile.value(),
        }
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

impl From<TrendSignal> for TrendSignalRequest {
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

impl From<TrendRegime> for TrendRegimeRequest {
    fn from(value: TrendRegime) -> Self {
        match value {
            TrendRegime::Overheated => Self::Overheated,
            TrendRegime::Neutral => Self::Neutral,
            TrendRegime::FallingKnife => Self::FallingKnife,
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
async fn market_sentiment_for_decision(state: &ApiState) -> Option<MarketSentimentReport> {
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

/// Borrowed decision material required to create one local audit record.
struct DecisionRecordContext<'a> {
    plan_id: Uuid,
    input: &'a DecisionPreviewRequest,
    execution: &'a InvestmentPlanExecutionPreview,
    decision: &'a DecisionSignal,
    market_sentiment: Option<&'a MarketSentimentReport>,
    trigger: DecisionTrigger,
    paper_order: Option<&'a BrokerOrderRequest>,
    paper_order_ack: Option<&'a BrokerOrderAck>,
    summary: String,
}

/// Build a complete local audit snapshot before any optional broker side effect.
fn record_input(context: DecisionRecordContext<'_>) -> Result<CreateDecisionRecord, ApiError> {
    Ok(CreateDecisionRecord {
        plan_id: context.plan_id,
        symbol: context.execution.symbol.clone(),
        currency: context.execution.currency.clone(),
        execution_status: execution_status(context.execution.status),
        planned_contribution: context
            .execution
            .planned_contribution
            .map(|value| value.to_string()),
        execution_snapshot: json!({
            "trigger": trigger_label(context.trigger),
            "execution": snapshot(context.execution)?,
        }),
        fundamental_snapshot: signal_snapshot(
            "fundamental",
            &context.input.fundamental,
            context.input.input_source.as_ref(),
        )?,
        trend_snapshot: signal_snapshot(
            "trend",
            &context.input.trend,
            context.input.input_source.as_ref(),
        )?,
        sentiment_snapshot: context.market_sentiment.map(market_sentiment_snapshot),
        decision_snapshot: decision_snapshot(context.decision),
        broker_order_request: context.paper_order.map(snapshot).transpose()?,
        broker_order_ack: context.paper_order_ack.map(snapshot).transpose()?,
        summary: context.summary,
    })
}

/// Build a source-labelled 70% or 20% audit snapshot without retaining credentials.
fn signal_snapshot(
    layer: &'static str,
    signal: &impl Serialize,
    automatic_source: Option<&Value>,
) -> Result<Value, ApiError> {
    Ok(json!({
        "layer": layer,
        "source": automatic_source.cloned().unwrap_or_else(|| json!({
            "kind": "operator_input",
            "description": "validated values supplied through the legacy decision-preview endpoint",
        })),
        "signal": snapshot(signal)?,
    }))
}

/// Build the auditable, non-secret provider disclosure for automatic 70/20 inputs.
fn automatic_source_snapshot(input: &MarketSignalInput) -> Value {
    json!({
        "kind": "automatic_market_data",
        "symbol": input.symbol,
        "as_of": input.as_of,
        "fundamental": "Shiller CAPE monthly table; ERP proxy = 100 / CAPE - US Treasury 10-year yield",
        "trend": "local OpenD daily close with locally computed MA200 and RSI; Cboe VIX monthly last observation",
    })
}

/// Return a stable trigger label for a persisted execution snapshot.
fn trigger_label(trigger: DecisionTrigger) -> &'static str {
    match trigger {
        DecisionTrigger::ManualInput => "manual_input",
        DecisionTrigger::AutomaticPreview => "automatic_preview",
        DecisionTrigger::AutomaticScheduler => "automatic_scheduler",
    }
}

/// Return a stable audit snapshot for an automatically retrieved market sentiment.
fn market_sentiment_snapshot(report: &MarketSentimentReport) -> Value {
    json!({
        "source": "market_sentiment",
        "score": report.analysis.sentiment().value(),
        "rationale": report.analysis.rationale(),
        "warnings": report.analysis.warnings(),
        "headlines": report.headlines.iter().map(|headline| json!({
            "title": headline.title,
            "url": headline.url,
            "published_at": headline.published_at.to_rfc3339(),
        })).collect::<Vec<_>>(),
    })
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
