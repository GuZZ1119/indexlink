#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Reproducible, offline-only calibration of the current IndexLink strategy.
//!
//! This crate is intentionally separate from production composition roots. It
//! reads only committed fixture data, invokes the real quant, decision, and
//! two-bucket domain functions, and never performs network or broker IO.

use std::collections::BTreeMap;

use ai_client::Sentiment;
use chrono::NaiveDate;
use decision_engine::{
    evaluate_decision, DecisionConfig, DecisionInput, DecisionSentiment, DecisionSignal,
};
use investment_plans::{
    BucketAllocationRatio, OpportunityCashPolicy, PlanExecutionConfiguration, PlanRiskMode,
    TwoBucketAllocationConfig, TwoBucketContributionSplit,
};
use quant_engine::{
    evaluate_fundamental, evaluate_trend, FundamentalConfig, FundamentalSignal,
    FundamentalSnapshot, TrendSignal, TrendSnapshot,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const PERIOD_BUDGET: i64 = 1_000;
const MAX_SINGLE_EXECUTION: i64 = 1_500;
const CORE_RATIO: i64 = 7;
const OPPORTUNITY_RATIO: i64 = 3;
const BUY_COST_BPS: f64 = 5.0;
const OOS_WINDOW_MONTHS: usize = 24;
const OOS_STEP_MONTHS: usize = 12;

/// Errors returned while reading or evaluating the committed calibration fixture.
#[derive(Debug, Error)]
pub enum EvaluationError {
    /// The committed fixture JSON is malformed.
    #[error("calibration fixture is invalid")]
    Fixture(#[from] serde_json::Error),
    /// A fixture date cannot be parsed as ISO `YYYY-MM-DD`.
    #[error("calibration fixture contains an invalid date")]
    InvalidDate,
    /// The fixture does not contain enough earlier monthly observations.
    #[error("calibration fixture has insufficient history")]
    InsufficientHistory,
    /// One of the real quant functions rejected a fixture observation.
    #[error(transparent)]
    Quant(#[from] quant_engine::QuantError),
    /// The two-bucket domain function rejected a fixed baseline configuration.
    #[error(transparent)]
    Plan(#[from] investment_plans::PlanValidationError),
}

/// A complete machine-readable result for calibration-v1.
#[derive(Debug, Serialize)]
pub struct CalibrationReport {
    dataset_version: String,
    assumptions: Assumptions,
    assets: Vec<AssetReport>,
    qwen_sensitivity: QwenSensitivityReport,
}

/// Serialize an evaluation report as deterministic human-readable JSON.
pub fn report_json(report: &CalibrationReport) -> Result<String, EvaluationError> {
    serde_json::to_string_pretty(report).map_err(EvaluationError::from)
}

/// Evaluate the committed calibration-v1 fixture using the unmodified production defaults.
pub fn evaluate_fixture() -> Result<CalibrationReport, EvaluationError> {
    let dataset: FixtureDataset =
        serde_json::from_str(include_str!("../data/generated/calibration-v1.json"))?;
    let configuration = execution_configuration()?;
    let mut evaluated_assets = Vec::new();
    let mut fallback_samples = Vec::new();

    for asset in &dataset.assets {
        let samples = evaluate_asset(asset)?;
        fallback_samples.extend(samples.iter().cloned());
        evaluated_assets.push(asset_report(asset, &samples, configuration)?);
    }

    Ok(CalibrationReport {
        dataset_version: dataset.dataset_version,
        assumptions: Assumptions {
            contribution_schedule: "monthly, last available observation for each asset".to_owned(),
            period_budget_usd: PERIOD_BUDGET as f64,
            core_ratio: CORE_RATIO as f64 / 10.0,
            opportunity_ratio: OPPORTUNITY_RATIO as f64 / 10.0,
            buy_cost_bps: BUY_COST_BPS,
            cost_model: "Each strategy buys at close × (1 + buy_cost_bps / 10,000); cash, including unallocated opportunity cash, remains in terminal wealth and earns no interest.".to_owned(),
            historical_ai_policy: "Historical causal results use DecisionSentiment::Unavailable and the current 90/10/0 fallback.".to_owned(),
        },
        assets: evaluated_assets,
        qwen_sensitivity: qwen_sensitivity(&fallback_samples),
    })
}

#[derive(Debug, Deserialize)]
struct FixtureDataset {
    dataset_version: String,
    assets: Vec<FixtureAsset>,
}

#[derive(Debug, Deserialize)]
struct FixtureAsset {
    id: String,
    display_name: String,
    source_symbol: String,
    observations: Vec<FixtureObservation>,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureObservation {
    as_of: String,
    close: f64,
    cape: f64,
    erp_proxy: f64,
    ma200_distance: f64,
    rsi14: f64,
    vix: f64,
}

#[derive(Debug, Serialize)]
struct Assumptions {
    contribution_schedule: String,
    period_budget_usd: f64,
    core_ratio: f64,
    opportunity_ratio: f64,
    buy_cost_bps: f64,
    cost_model: String,
    historical_ai_policy: String,
}

#[derive(Debug, Serialize)]
struct AssetReport {
    asset_id: String,
    display_name: String,
    source_symbol: String,
    decision_observations: usize,
    first_decision_date: String,
    last_decision_date: String,
    fallback_score_distribution: Distribution,
    fallback_action_distribution: BTreeMap<String, u32>,
    fallback_layer_calibration: LayerCalibration,
    core_opportunity_intent: PerformanceMetrics,
    current_api_effective: PerformanceMetrics,
    fixed_dca: PerformanceMetrics,
    intent_vs_dca_terminal_difference_percent: f64,
    current_api_vs_intent_terminal_difference_percent: f64,
    rolling_out_of_sample: Vec<RollingWindow>,
    experimental_candidates: Vec<CandidateReport>,
}

#[derive(Debug, Serialize)]
struct Distribution {
    mean: f64,
    median: f64,
    p10: f64,
    p25: f64,
    p75: f64,
    p90: f64,
}

#[derive(Debug, Serialize)]
struct LayerCalibration {
    fundamental_raw_mean: f64,
    fundamental_directional_mean: f64,
    fundamental_weighted_contribution_mean: f64,
    trend_raw_mean: f64,
    trend_timing_mean: f64,
    trend_weighted_contribution_mean: f64,
    sentiment_input_mean_when_unavailable: f64,
    sentiment_weighted_contribution_mean: f64,
    final_score_mean: f64,
}

#[derive(Debug, Serialize)]
struct PerformanceMetrics {
    xirr_percent: Option<f64>,
    terminal_wealth_usd: f64,
    maximum_drawdown_percent: f64,
    annualized_volatility_percent: Option<f64>,
    total_external_cash_usd: f64,
    total_invested_usd: f64,
    cash_utilisation_percent: f64,
    terminal_cash_usd: f64,
    terminal_opportunity_cash_usd: f64,
}

#[derive(Debug, Serialize)]
struct RollingWindow {
    start: String,
    end: String,
    months: usize,
    core_opportunity_intent: PerformanceMetrics,
    fixed_dca: PerformanceMetrics,
    terminal_difference_percent: f64,
}

#[derive(Debug, Serialize)]
struct CandidateReport {
    id: String,
    status: String,
    rule: String,
    performance: PerformanceMetrics,
    terminal_difference_vs_dca_percent: f64,
    rolling_out_of_sample: Vec<CandidateRollingWindow>,
}

#[derive(Debug, Serialize)]
struct CandidateRollingWindow {
    start: String,
    end: String,
    months: usize,
    terminal_difference_vs_dca_percent: f64,
}

#[derive(Debug, Serialize)]
struct QwenSensitivityReport {
    scope: String,
    frozen_score_version: String,
    sample_count: usize,
    fallback_action_distribution: BTreeMap<String, u32>,
    normal_70_20_10_action_distribution: BTreeMap<String, u32>,
    fallback_score_distribution: Distribution,
    normal_70_20_10_score_distribution: Distribution,
    mean_normal_minus_fallback_score: f64,
}

#[derive(Debug, Deserialize)]
struct FrozenQwenSensitivity {
    version: String,
    purpose: String,
    scores: Vec<f64>,
}

#[derive(Clone)]
struct DecisionMonth {
    date: NaiveDate,
    close: f64,
    fundamental: FundamentalSignal,
    trend: TrendSignal,
    fallback: DecisionSignal,
}

fn execution_configuration() -> Result<PlanExecutionConfiguration, EvaluationError> {
    let allocation = TwoBucketAllocationConfig::new(
        BucketAllocationRatio::new(Decimal::new(CORE_RATIO, 1))?,
        BucketAllocationRatio::new(Decimal::new(OPPORTUNITY_RATIO, 1))?,
    )?;
    PlanExecutionConfiguration::new_with_cash_policy(
        allocation,
        PlanRiskMode::Autopilot,
        OpportunityCashPolicy::CarryForward,
    )
    .map_err(EvaluationError::from)
}

fn evaluate_asset(asset: &FixtureAsset) -> Result<Vec<DecisionMonth>, EvaluationError> {
    let mut output = Vec::new();
    for index in 60..asset.observations.len() {
        let current = &asset.observations[index];
        let history = &asset.observations[..index];
        let fundamental = evaluate_fundamental(
            &FundamentalSnapshot {
                cape_history: history.iter().map(|row| row.cape).collect(),
                cape_current: current.cape,
                erp_history: history.iter().map(|row| row.erp_proxy).collect(),
                erp_current: current.erp_proxy,
            },
            &FundamentalConfig::default(),
        )?;
        let trend = evaluate_trend(
            &TrendSnapshot {
                ma_distance_history: history.iter().map(|row| row.ma200_distance).collect(),
                ma_distance_current: current.ma200_distance,
                rsi_history: history.iter().map(|row| row.rsi14).collect(),
                rsi_current: current.rsi14,
                vix_history: history.iter().map(|row| row.vix).collect(),
                vix_current: current.vix,
            },
            &quant_engine::TrendConfig::default(),
        )?;
        let fallback = evaluate_decision(
            &DecisionInput {
                fundamental: fundamental.clone(),
                trend: trend.clone(),
                sentiment: DecisionSentiment::Unavailable,
            },
            &DecisionConfig::default(),
        );
        output.push(DecisionMonth {
            date: NaiveDate::parse_from_str(&current.as_of, "%Y-%m-%d")
                .map_err(|_| EvaluationError::InvalidDate)?,
            close: current.close,
            fundamental,
            trend,
            fallback,
        });
    }
    (!output.is_empty())
        .then_some(output)
        .ok_or(EvaluationError::InsufficientHistory)
}

fn asset_report(
    asset: &FixtureAsset,
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
) -> Result<AssetReport, EvaluationError> {
    let intent = simulate(samples, configuration, ExecutionMode::CoreOpportunityIntent)?;
    let current_api = simulate(samples, configuration, ExecutionMode::CurrentApiEffective)?;
    let dca = simulate(samples, configuration, ExecutionMode::FixedDca)?;
    let first = samples
        .first()
        .ok_or(EvaluationError::InsufficientHistory)?;
    let last = samples.last().ok_or(EvaluationError::InsufficientHistory)?;
    let rolling_out_of_sample = (0..samples.len())
        .step_by(OOS_STEP_MONTHS)
        .filter_map(|start| samples.get(start..start + OOS_WINDOW_MONTHS))
        .map(|window| {
            let intent = simulate(window, configuration, ExecutionMode::CoreOpportunityIntent)?;
            let dca = simulate(window, configuration, ExecutionMode::FixedDca)?;
            Ok(RollingWindow {
                start: window[0].date.to_string(),
                end: window[window.len() - 1].date.to_string(),
                months: window.len(),
                terminal_difference_percent: relative_difference(
                    intent.terminal_wealth_usd,
                    dca.terminal_wealth_usd,
                ),
                core_opportunity_intent: intent,
                fixed_dca: dca,
            })
        })
        .collect::<Result<Vec<_>, EvaluationError>>()?;
    let experimental_candidates = vec![bounded_continuous_candidate(samples, configuration, &dca)?];

    Ok(AssetReport {
        asset_id: asset.id.clone(),
        display_name: asset.display_name.clone(),
        source_symbol: asset.source_symbol.clone(),
        decision_observations: samples.len(),
        first_decision_date: first.date.to_string(),
        last_decision_date: last.date.to_string(),
        fallback_score_distribution: distribution(
            samples.iter().map(|row| row.fallback.final_score.value()),
        ),
        fallback_action_distribution: action_distribution(samples.iter().map(|row| &row.fallback)),
        fallback_layer_calibration: layer_calibration(samples),
        intent_vs_dca_terminal_difference_percent: relative_difference(
            intent.terminal_wealth_usd,
            dca.terminal_wealth_usd,
        ),
        current_api_vs_intent_terminal_difference_percent: relative_difference(
            current_api.terminal_wealth_usd,
            intent.terminal_wealth_usd,
        ),
        core_opportunity_intent: intent,
        current_api_effective: current_api,
        fixed_dca: dca,
        rolling_out_of_sample,
        experimental_candidates,
    })
}

#[derive(Clone, Copy)]
enum ExecutionMode {
    FixedDca,
    CoreOpportunityIntent,
    CurrentApiEffective,
    CandidateBoundedContinuous,
}

fn simulate(
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
    mode: ExecutionMode,
) -> Result<PerformanceMetrics, EvaluationError> {
    let budget = Decimal::new(PERIOD_BUDGET, 0);
    let maximum = Decimal::new(MAX_SINGLE_EXECUTION, 0);
    let mut opportunity_cash = Decimal::ZERO;
    let mut state = PortfolioState::default();
    for row in samples {
        state.deposit(row.date, PERIOD_BUDGET as f64);
        let (action, multiplier) = match mode {
            ExecutionMode::CandidateBoundedContinuous => {
                let multiplier = core_domain::Multiplier::new_clamped(
                    0.75 + 0.5 * row.fallback.final_score.value(),
                );
                (multiplier.to_action(), multiplier)
            }
            _ => (row.fallback.action, row.fallback.multiplier),
        };
        let intended = TwoBucketContributionSplit::from_decision_with_carry(
            budget,
            maximum,
            configuration,
            action,
            multiplier,
            opportunity_cash,
        )?;
        let spend = match mode {
            ExecutionMode::FixedDca => budget,
            ExecutionMode::CoreOpportunityIntent => intended.recommended_contribution(),
            ExecutionMode::CurrentApiEffective => {
                if matches!(
                    action,
                    core_domain::Action::Skip | core_domain::Action::TacticalDelay
                ) {
                    Decimal::ZERO
                } else {
                    intended.recommended_contribution()
                }
            }
            ExecutionMode::CandidateBoundedContinuous => intended.recommended_contribution(),
        };
        if !matches!(mode, ExecutionMode::FixedDca) {
            opportunity_cash = (opportunity_cash + intended.opportunity_budget()
                - intended.opportunity_contribution())
            .max(Decimal::ZERO);
        }
        let spend = spend.to_f64().ok_or(EvaluationError::InsufficientHistory)?;
        state.buy(row.date, spend, row.close);
        state.mark_to_market(row.date, row.close);
    }
    Ok(state.metrics(opportunity_cash.to_f64().unwrap_or_default()))
}

fn bounded_continuous_candidate(
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
    fixed_dca: &PerformanceMetrics,
) -> Result<CandidateReport, EvaluationError> {
    let performance = simulate(
        samples,
        configuration,
        ExecutionMode::CandidateBoundedContinuous,
    )?;
    let rolling_out_of_sample = (0..samples.len())
        .step_by(OOS_STEP_MONTHS)
        .filter_map(|start| samples.get(start..start + OOS_WINDOW_MONTHS))
        .map(|window| {
            let candidate = simulate(
                window,
                configuration,
                ExecutionMode::CandidateBoundedContinuous,
            )?;
            let dca = simulate(window, configuration, ExecutionMode::FixedDca)?;
            Ok(CandidateRollingWindow {
                start: window[0].date.to_string(),
                end: window[window.len() - 1].date.to_string(),
                months: window.len(),
                terminal_difference_vs_dca_percent: relative_difference(
                    candidate.terminal_wealth_usd,
                    dca.terminal_wealth_usd,
                ),
            })
        })
        .collect::<Result<Vec<_>, EvaluationError>>()?;
    Ok(CandidateReport {
        id: "bounded_continuous_opportunity_v1".to_owned(),
        status: "experimental; not a production default".to_owned(),
        rule: "Keep the 70% core bucket; replace only opportunity execution with multiplier = 0.75 + 0.50 × current final_score, bounded to [0.75, 1.25]. Do not turn non-neutral trend regimes into a global order veto in this evaluation-only candidate.".to_owned(),
        terminal_difference_vs_dca_percent: relative_difference(
            performance.terminal_wealth_usd,
            fixed_dca.terminal_wealth_usd,
        ),
        performance,
        rolling_out_of_sample,
    })
}

struct PortfolioState {
    cash: f64,
    units: f64,
    external_cash: f64,
    invested: f64,
    flows: Vec<(NaiveDate, f64)>,
    last_value: f64,
    pending_external_flow: f64,
    time_weighted_nav: f64,
    nav_values: Vec<f64>,
    period_returns: Vec<f64>,
}

impl Default for PortfolioState {
    fn default() -> Self {
        Self {
            cash: 0.0,
            units: 0.0,
            external_cash: 0.0,
            invested: 0.0,
            flows: Vec::new(),
            last_value: 0.0,
            pending_external_flow: 0.0,
            time_weighted_nav: 1.0,
            nav_values: Vec::new(),
            period_returns: Vec::new(),
        }
    }
}

impl PortfolioState {
    fn deposit(&mut self, date: NaiveDate, amount: f64) {
        self.cash += amount;
        self.external_cash += amount;
        self.pending_external_flow += amount;
        self.flows.push((date, -amount));
    }

    fn buy(&mut self, _date: NaiveDate, amount: f64, close: f64) {
        let amount = amount.min(self.cash).max(0.0);
        self.cash -= amount;
        self.invested += amount;
        self.units += amount / (close * (1.0 + BUY_COST_BPS / 10_000.0));
    }

    fn mark_to_market(&mut self, _date: NaiveDate, close: f64) {
        let value = self.cash + self.units * close;
        let denominator = self.last_value + self.pending_external_flow;
        if denominator > 0.0 {
            let period_return = value / denominator - 1.0;
            self.time_weighted_nav *= 1.0 + period_return;
            self.nav_values.push(self.time_weighted_nav);
            if self.last_value > 0.0 {
                self.period_returns.push(period_return);
            }
        }
        self.last_value = value;
        self.pending_external_flow = 0.0;
    }

    fn metrics(mut self, terminal_opportunity_cash_usd: f64) -> PerformanceMetrics {
        let terminal = self.last_value;
        if let Some(last_flow) = self.flows.last_mut() {
            last_flow.1 += terminal;
        }
        PerformanceMetrics {
            xirr_percent: xirr(&self.flows).map(|value| value * 100.0),
            terminal_wealth_usd: terminal,
            maximum_drawdown_percent: maximum_drawdown(&self.nav_values) * 100.0,
            annualized_volatility_percent: annualized_volatility(&self.period_returns)
                .map(|value| value * 100.0),
            total_external_cash_usd: self.external_cash,
            total_invested_usd: self.invested,
            cash_utilisation_percent: if self.external_cash == 0.0 {
                0.0
            } else {
                self.invested / self.external_cash * 100.0
            },
            terminal_cash_usd: self.cash,
            terminal_opportunity_cash_usd,
        }
    }
}

fn qwen_sensitivity(samples: &[DecisionMonth]) -> QwenSensitivityReport {
    let frozen: FrozenQwenSensitivity =
        serde_json::from_str(include_str!("../data/generated/qwen-sensitivity-v1.json"))
            .expect("the committed Qwen sensitivity fixture must be valid JSON");
    let normal: Vec<_> = samples
        .iter()
        .enumerate()
        .map(|(index, row)| {
            evaluate_decision(
                &DecisionInput {
                    fundamental: row.fundamental.clone(),
                    trend: row.trend.clone(),
                    sentiment: DecisionSentiment::Available(
                        Sentiment::new(frozen.scores[index % frozen.scores.len()])
                            .expect("frozen sensitivity scores are bounded"),
                    ),
                },
                &DecisionConfig::default(),
            )
        })
        .collect();
    let fallback_scores: Vec<_> = samples
        .iter()
        .map(|row| row.fallback.final_score.value())
        .collect();
    let normal_scores: Vec<_> = normal.iter().map(|row| row.final_score.value()).collect();
    QwenSensitivityReport {
        scope: frozen.purpose,
        frozen_score_version: frozen.version,
        sample_count: samples.len(),
        fallback_action_distribution: action_distribution(samples.iter().map(|row| &row.fallback)),
        normal_70_20_10_action_distribution: action_distribution(normal.iter()),
        fallback_score_distribution: distribution(fallback_scores.clone()),
        normal_70_20_10_score_distribution: distribution(normal_scores.clone()),
        mean_normal_minus_fallback_score: mean(normal_scores) - mean(fallback_scores),
    }
}

fn distribution(values: impl IntoIterator<Item = f64>) -> Distribution {
    let mut values: Vec<_> = values.into_iter().collect();
    values.sort_by(f64::total_cmp);
    Distribution {
        mean: mean(values.clone()),
        median: percentile(&values, 0.5),
        p10: percentile(&values, 0.1),
        p25: percentile(&values, 0.25),
        p75: percentile(&values, 0.75),
        p90: percentile(&values, 0.9),
    }
}

fn percentile(values: &[f64], probability: f64) -> f64 {
    let index = ((values.len().saturating_sub(1)) as f64 * probability).round() as usize;
    values.get(index).copied().unwrap_or_default()
}

fn mean(values: impl IntoIterator<Item = f64>) -> f64 {
    let values: Vec<_> = values.into_iter().collect();
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn action_distribution<'a>(
    signals: impl IntoIterator<Item = &'a DecisionSignal>,
) -> BTreeMap<String, u32> {
    signals
        .into_iter()
        .fold(BTreeMap::new(), |mut output, signal| {
            *output.entry(format!("{:?}", signal.action)).or_default() += 1;
            output
        })
}

fn layer_calibration(samples: &[DecisionMonth]) -> LayerCalibration {
    let values = samples.iter().map(|row| {
        let signal = &row.fallback;
        (
            row.fundamental.score.value(),
            signal.fundamental_score.value(),
            signal.weights.fundamental_weight.value() * signal.fundamental_score.value(),
            row.trend.score.value(),
            signal.trend_score.value(),
            signal.weights.trend_weight.value() * signal.trend_score.value(),
            signal.sentiment_score.map_or(0.5, |value| value.value()),
            signal.weights.sentiment_weight.value()
                * signal.sentiment_score.map_or(0.5, |value| value.value()),
            signal.final_score.value(),
        )
    });
    let mut fundamental_raw = Vec::new();
    let mut fundamental_directional = Vec::new();
    let mut fundamentals = Vec::new();
    let mut trend_raw = Vec::new();
    let mut trend_timing = Vec::new();
    let mut trends = Vec::new();
    let mut sentiment_input = Vec::new();
    let mut sentiments = Vec::new();
    let mut finals = Vec::new();
    for (
        raw_fundamental,
        directional_fundamental,
        fundamental,
        raw_trend,
        timing_trend,
        trend,
        sentiment_value,
        sentiment,
        final_score,
    ) in values
    {
        fundamental_raw.push(raw_fundamental);
        fundamental_directional.push(directional_fundamental);
        fundamentals.push(fundamental);
        trend_raw.push(raw_trend);
        trend_timing.push(timing_trend);
        trends.push(trend);
        sentiment_input.push(sentiment_value);
        sentiments.push(sentiment);
        finals.push(final_score);
    }
    LayerCalibration {
        fundamental_raw_mean: mean(fundamental_raw),
        fundamental_directional_mean: mean(fundamental_directional),
        fundamental_weighted_contribution_mean: mean(fundamentals),
        trend_raw_mean: mean(trend_raw),
        trend_timing_mean: mean(trend_timing),
        trend_weighted_contribution_mean: mean(trends),
        sentiment_input_mean_when_unavailable: mean(sentiment_input),
        sentiment_weighted_contribution_mean: mean(sentiments),
        final_score_mean: mean(finals),
    }
}

fn relative_difference(actual: f64, benchmark: f64) -> f64 {
    if benchmark == 0.0 {
        0.0
    } else {
        (actual / benchmark - 1.0) * 100.0
    }
}

fn maximum_drawdown(values: &[f64]) -> f64 {
    let mut peak = 0.0_f64;
    values.iter().fold(0.0_f64, |maximum, value| {
        peak = peak.max(*value);
        if peak == 0.0 {
            maximum
        } else {
            maximum.max((peak - value) / peak)
        }
    })
}

fn annualized_volatility(returns: &[f64]) -> Option<f64> {
    if returns.len() < 2 {
        return None;
    }
    let average = mean(returns.iter().copied());
    let variance = returns
        .iter()
        .map(|value| (value - average).powi(2))
        .sum::<f64>()
        / (returns.len() - 1) as f64;
    Some(variance.sqrt() * 12.0_f64.sqrt())
}

fn xirr(flows: &[(NaiveDate, f64)]) -> Option<f64> {
    let first = flows.first()?.0;
    let npv = |rate: f64| -> f64 {
        flows
            .iter()
            .map(|(date, cash)| {
                let years = (*date - first).num_days() as f64 / 365.25;
                cash / (1.0 + rate).powf(years)
            })
            .sum()
    };
    let mut lower = -0.9999;
    let mut upper = 10.0;
    let lower_value = npv(lower);
    let upper_value = npv(upper);
    if lower_value.signum() == upper_value.signum() {
        return None;
    }
    for _ in 0..100 {
        let middle = (lower + upper) / 2.0;
        if npv(middle).signum() == lower_value.signum() {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    Some((lower + upper) / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the committed fixture produces a deterministic non-empty offline report.
    #[test]
    fn fixture_evaluation_is_reproducible() {
        let first = report_json(&evaluate_fixture().unwrap()).unwrap();
        let second = report_json(&evaluate_fixture().unwrap()).unwrap();
        assert_eq!(first, second);
        assert!(first.contains("calibration-v1"));
        assert!(first.contains("sp500_index_proxy"));
        assert!(first.contains("nasdaq_composite_proxy"));
    }

    /// Verify every score is built from earlier observations only.
    #[test]
    fn decisions_require_sixty_prior_observations() {
        let dataset: FixtureDataset =
            serde_json::from_str(include_str!("../data/generated/calibration-v1.json")).unwrap();
        for asset in dataset.assets {
            let samples = evaluate_asset(&asset).unwrap();
            assert_eq!(samples.len(), asset.observations.len() - 60);
        }
    }

    /// Verify the core/opportunity simulation treats retained money as terminal cash.
    #[test]
    fn strategy_never_spends_more_than_external_cash() {
        let report = report_json(&evaluate_fixture().unwrap()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        for asset in value["assets"].as_array().unwrap() {
            let metrics = &asset["core_opportunity_intent"];
            assert!(
                metrics["total_invested_usd"].as_f64().unwrap()
                    <= metrics["total_external_cash_usd"].as_f64().unwrap()
            );
        }
    }
}
