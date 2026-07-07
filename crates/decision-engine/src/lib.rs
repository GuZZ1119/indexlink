#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! 70/20/10 decision engine.
//!
//! This crate is pure domain logic: it combines quant signals and bounded AI
//! sentiment into a final score, multiplier, and action. It performs no IO and
//! does not know about HTTP, storage, or broker adapters.

use ai_client::Sentiment;
use core_domain::{Action, Multiplier, Percentile};
use quant_engine::{FundamentalSignal, TrendRegime, TrendSignal, Weight};

const WEIGHT_SUM_TOLERANCE: f64 = 1e-9;
const NEUTRAL_SENTIMENT_SCORE: f64 = 0.5;

/// Default fundamental layer weight in the normal 70/20/10 model.
pub const DEFAULT_FUNDAMENTAL_WEIGHT: f64 = 0.70;
/// Default trend layer weight in the normal 70/20/10 model.
pub const DEFAULT_TREND_WEIGHT: f64 = 0.20;
/// Default AI sentiment layer weight in the normal 70/20/10 model.
pub const DEFAULT_SENTIMENT_WEIGHT: f64 = 0.10;
/// Fundamental layer weight used when AI sentiment is unavailable.
pub const FALLBACK_FUNDAMENTAL_WEIGHT: f64 = 0.90;
/// Trend layer weight used when AI sentiment is unavailable.
pub const FALLBACK_TREND_WEIGHT: f64 = 0.10;
/// AI sentiment layer weight used when AI sentiment is unavailable.
pub const FALLBACK_SENTIMENT_WEIGHT: f64 = 0.0;

/// Decision-layer weights for fundamental, trend, and sentiment inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionWeights {
    /// Fundamental layer weight.
    pub fundamental_weight: Weight,
    /// Trend layer weight.
    pub trend_weight: Weight,
    /// Sentiment layer weight.
    pub sentiment_weight: Weight,
}

impl DecisionWeights {
    /// Build validated decision weights.
    ///
    /// Each weight must be in `[0.0, 1.0]`, and the three weights must sum to
    /// one within a small floating-point tolerance.
    pub fn new(fundamental: f64, trend: f64, sentiment: f64) -> Result<Self, DecisionError> {
        let fundamental_weight = Weight::new(fundamental)?;
        let trend_weight = Weight::new(trend)?;
        let sentiment_weight = Weight::new(sentiment)?;
        let sum = fundamental + trend + sentiment;

        if (sum - 1.0).abs() > WEIGHT_SUM_TOLERANCE {
            return Err(DecisionError::InvalidWeightSum { sum });
        }

        Ok(Self {
            fundamental_weight,
            trend_weight,
            sentiment_weight,
        })
    }
}

impl Default for DecisionWeights {
    fn default() -> Self {
        Self::new(
            DEFAULT_FUNDAMENTAL_WEIGHT,
            DEFAULT_TREND_WEIGHT,
            DEFAULT_SENTIMENT_WEIGHT,
        )
        .expect("default decision weights sum to 1.0")
    }
}

/// Decision engine configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionConfig {
    /// Normal 70/20/10 weights used when sentiment is available.
    pub weights: DecisionWeights,
    /// Fallback weights used when sentiment is unavailable.
    pub sentiment_unavailable_weights: DecisionWeights,
}

impl DecisionConfig {
    /// Build a decision configuration from normal and fallback weights.
    #[must_use]
    pub fn new(weights: DecisionWeights, sentiment_unavailable_weights: DecisionWeights) -> Self {
        Self {
            weights,
            sentiment_unavailable_weights,
        }
    }
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            weights: DecisionWeights::default(),
            sentiment_unavailable_weights: DecisionWeights::new(
                FALLBACK_FUNDAMENTAL_WEIGHT,
                FALLBACK_TREND_WEIGHT,
                FALLBACK_SENTIMENT_WEIGHT,
            )
            .expect("default fallback decision weights sum to 1.0"),
        }
    }
}

/// Sentiment input passed into the decision engine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecisionSentiment {
    /// AI sentiment is available and should contribute to the normal weights.
    Available(Sentiment),
    /// AI sentiment is unavailable; the engine should use fallback weights.
    Unavailable,
}

/// Input snapshot for one decision evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionInput {
    /// Fundamental signal from the 70% quant layer.
    pub fundamental: FundamentalSignal,
    /// Trend signal from the 20% quant layer.
    pub trend: TrendSignal,
    /// AI sentiment input, or an explicit unavailable marker.
    pub sentiment: DecisionSentiment,
}

/// Whether the decision used normal 70/20/10 weights or a degraded fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionWeightMode {
    /// Sentiment was available, so the normal 70/20/10 weights were used.
    Normal,
    /// Sentiment was unavailable, so fallback weights were used.
    SentimentUnavailable,
}

/// Decision result returned by the engine.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionSignal {
    /// Original input snapshot used to produce this decision.
    ///
    /// Keeping the raw input alongside derived scores makes later audit,
    /// storage, and replay flows traceable without relying on callers to
    /// persist a separate object.
    pub input: DecisionInput,
    /// Final investability score in `[0.0, 1.0]`.
    ///
    /// Higher values mean the engine is more willing to increase the planned
    /// contribution, before discrete risk actions such as tactical delay.
    pub final_score: Percentile,
    /// Final contribution multiplier, clamped by [`Multiplier`].
    pub multiplier: Multiplier,
    /// Final action label derived from multiplier and trend regime.
    pub action: Action,
    /// Weights actually used by this evaluation.
    pub weights: DecisionWeights,
    /// Whether normal or fallback weights were used.
    pub weight_mode: DecisionWeightMode,
    /// Fundamental layer contribution after direction normalization.
    pub fundamental_score: Percentile,
    /// Trend timing contribution after safety normalization.
    pub trend_score: Percentile,
    /// Sentiment contribution after mapping `[-1.0, 1.0]` to `[0.0, 1.0]`.
    pub sentiment_score: Option<Percentile>,
}

/// Decision engine error.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DecisionError {
    /// A decision weight was outside the valid `[0.0, 1.0]` range.
    #[error(transparent)]
    InvalidWeight(#[from] quant_engine::QuantError),
    /// Decision weights did not sum to one.
    #[error("decision weights must sum to 1.0, got {sum}")]
    InvalidWeightSum {
        /// Actual sum of the configured weights.
        sum: f64,
    },
}

/// Evaluate one 70/20/10 decision.
///
/// Direction normalization:
/// - fundamental score is inverted, because lower valuation is more attractive;
/// - trend score rewards neutral timing and penalizes overheat/falling-knife extremes;
/// - sentiment maps `[-1.0, 1.0]` into `[0.0, 1.0]`.
/// - unavailable sentiment contributes neutral `0.5` if a custom fallback still
///   assigns non-zero sentiment weight.
#[must_use]
pub fn evaluate_decision(input: &DecisionInput, config: &DecisionConfig) -> DecisionSignal {
    let (weights, weight_mode, sentiment_score) = match input.sentiment {
        DecisionSentiment::Available(sentiment) => (
            config.weights.clone(),
            DecisionWeightMode::Normal,
            Some(sentiment_to_score(sentiment)),
        ),
        DecisionSentiment::Unavailable => (
            config.sentiment_unavailable_weights.clone(),
            DecisionWeightMode::SentimentUnavailable,
            None,
        ),
    };

    let fundamental_score = input.fundamental.score.invert();
    let trend_score = trend_timing_score(input.trend.score);
    let sentiment_value = sentiment_score.map_or(NEUTRAL_SENTIMENT_SCORE, Percentile::value);

    let composite = weights.fundamental_weight.value() * fundamental_score.value()
        + weights.trend_weight.value() * trend_score.value()
        + weights.sentiment_weight.value() * sentiment_value;
    let final_score =
        Percentile::new(composite.clamp(0.0, 1.0)).expect("clamp keeps score in [0.0, 1.0]");
    let multiplier = multiplier_from_score(final_score);
    let action = if input.trend.regime == TrendRegime::Neutral {
        multiplier.to_action()
    } else {
        Action::TacticalDelay
    };

    DecisionSignal {
        input: input.clone(),
        final_score,
        multiplier,
        action,
        weights,
        weight_mode,
        fundamental_score,
        trend_score,
        sentiment_score,
    }
}

fn sentiment_to_score(sentiment: Sentiment) -> Percentile {
    Percentile::new((sentiment.value() + 1.0) / 2.0)
        .expect("sentiment score mapping stays in [0.0, 1.0]")
}

fn trend_timing_score(score: Percentile) -> Percentile {
    let distance_from_neutral = (score.value() - 0.5).abs();
    Percentile::new((0.5 - distance_from_neutral).clamp(0.0, 1.0))
        .expect("trend timing score stays in [0.0, 1.0]")
}

fn multiplier_from_score(score: Percentile) -> Multiplier {
    let raw = if score.value() <= 0.5 {
        score.value() * 2.0
    } else {
        1.0 + (score.value() - 0.5)
    };

    Multiplier::new_clamped(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn percentile(value: f64) -> Percentile {
        Percentile::new(value).unwrap()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    fn fundamental(score: f64) -> FundamentalSignal {
        FundamentalSignal {
            score: percentile(score),
            cape_percentile: percentile(score),
            erp_percentile: percentile(1.0 - score),
        }
    }

    fn trend(score: f64, regime: TrendRegime) -> TrendSignal {
        TrendSignal {
            score: percentile(score),
            ma_distance_percentile: percentile(0.5),
            rsi_percentile: percentile(0.5),
            vix_percentile: percentile(0.5),
            regime,
        }
    }

    fn input(
        fundamental_score: f64,
        trend_score: f64,
        regime: TrendRegime,
        sentiment: DecisionSentiment,
    ) -> DecisionInput {
        DecisionInput {
            fundamental: fundamental(fundamental_score),
            trend: trend(trend_score, regime),
            sentiment,
        }
    }

    /// Verify default weights represent the documented 70/20/10 split.
    #[test]
    fn default_config_uses_70_20_10_and_90_10_0_fallback() {
        let config = DecisionConfig::default();

        assert_eq!(config.weights.fundamental_weight.value(), 0.70);
        assert_eq!(config.weights.trend_weight.value(), 0.20);
        assert_eq!(config.weights.sentiment_weight.value(), 0.10);
        assert_eq!(
            config
                .sentiment_unavailable_weights
                .fundamental_weight
                .value(),
            0.90
        );
        assert_eq!(
            config.sentiment_unavailable_weights.trend_weight.value(),
            0.10
        );
        assert_eq!(
            config
                .sentiment_unavailable_weights
                .sentiment_weight
                .value(),
            0.0
        );
    }

    /// Verify invalid weight sums are rejected before evaluation.
    #[test]
    fn decision_weights_require_sum_of_one() {
        let err = DecisionWeights::new(0.7, 0.2, 0.2).unwrap_err();
        match err {
            DecisionError::InvalidWeightSum { sum } => assert_close(sum, 1.1),
            other => panic!("unexpected error: {other:?}"),
        }
        assert!(DecisionWeights::new(0.7, 0.2, 0.1).is_ok());
    }

    /// Verify neutral signals produce a standard 1.0x decision.
    #[test]
    fn neutral_inputs_produce_standard_action() {
        let result = evaluate_decision(
            &input(
                0.5,
                0.5,
                TrendRegime::Neutral,
                DecisionSentiment::Available(Sentiment::neutral()),
            ),
            &DecisionConfig::default(),
        );

        assert_close(result.final_score.value(), 0.5);
        assert_close(result.multiplier.value(), 1.0);
        assert_eq!(result.action, Action::Standard);
        assert_eq!(result.weight_mode, DecisionWeightMode::Normal);
        assert_eq!(result.input.fundamental.score.value(), 0.5);
        assert_eq!(result.input.trend.regime, TrendRegime::Neutral);
    }

    /// Verify cheap fundamentals and positive sentiment increase contribution.
    #[test]
    fn cheap_positive_neutral_timing_overweights() {
        let result = evaluate_decision(
            &input(
                0.1,
                0.5,
                TrendRegime::Neutral,
                DecisionSentiment::Available(Sentiment::new(0.8).unwrap()),
            ),
            &DecisionConfig::default(),
        );

        assert!(result.final_score.value() > 0.7);
        assert!(result.multiplier.value() > 1.2);
        assert_eq!(result.action, Action::Overweight);
    }

    /// Verify expensive fundamentals and negative sentiment reduce contribution.
    #[test]
    fn expensive_negative_inputs_underweight() {
        let result = evaluate_decision(
            &input(
                0.95,
                0.5,
                TrendRegime::Neutral,
                DecisionSentiment::Available(Sentiment::new(-0.8).unwrap()),
            ),
            &DecisionConfig::default(),
        );

        assert!(result.final_score.value() < 0.2);
        assert!(result.multiplier.value() < 0.75);
        assert_eq!(result.action, Action::Underweight);
    }

    /// Verify non-neutral trend regimes trigger tactical delay.
    #[test]
    fn trend_regime_triggers_tactical_delay() {
        for regime in [TrendRegime::Overheated, TrendRegime::FallingKnife] {
            let result = evaluate_decision(
                &input(
                    0.1,
                    0.0,
                    regime,
                    DecisionSentiment::Available(Sentiment::new(0.8).unwrap()),
                ),
                &DecisionConfig::default(),
            );

            assert_eq!(result.action, Action::TacticalDelay);
        }
    }

    /// Verify AI outages degrade from 70/20/10 to 90/10/0.
    #[test]
    fn unavailable_sentiment_uses_fallback_weights() {
        let result = evaluate_decision(
            &input(
                0.5,
                0.5,
                TrendRegime::Neutral,
                DecisionSentiment::Unavailable,
            ),
            &DecisionConfig::default(),
        );

        assert_eq!(result.weight_mode, DecisionWeightMode::SentimentUnavailable);
        assert_eq!(result.weights.fundamental_weight.value(), 0.90);
        assert_eq!(result.weights.trend_weight.value(), 0.10);
        assert_eq!(result.weights.sentiment_weight.value(), 0.0);
        assert_eq!(result.sentiment_score, None);
        assert_close(result.final_score.value(), 0.5);
    }

    /// Verify unavailable sentiment is neutral even with custom non-zero fallback weight.
    #[test]
    fn unavailable_sentiment_uses_neutral_value_for_custom_fallback_weights() {
        let config = DecisionConfig::new(
            DecisionWeights::default(),
            DecisionWeights::new(0.5, 0.0, 0.5).unwrap(),
        );
        let result = evaluate_decision(
            &input(
                0.5,
                0.0,
                TrendRegime::Neutral,
                DecisionSentiment::Unavailable,
            ),
            &config,
        );

        assert_eq!(result.sentiment_score, None);
        assert_close(result.final_score.value(), 0.5);
        assert_close(result.multiplier.value(), 1.0);
        assert_eq!(result.action, Action::Standard);
    }

    /// Verify extreme low scores can still skip the current execution.
    #[test]
    fn extreme_low_score_can_skip_execution() {
        let result = evaluate_decision(
            &input(
                1.0,
                0.0,
                TrendRegime::Neutral,
                DecisionSentiment::Available(Sentiment::MIN),
            ),
            &DecisionConfig::default(),
        );

        assert_close(result.final_score.value(), 0.0);
        assert_eq!(result.multiplier, Multiplier::MIN);
        assert_eq!(result.action, Action::Skip);
    }

    /// Verify the multiplier crosses the core-domain skip threshold smoothly.
    #[test]
    fn multiplier_mapping_is_continuous_around_skip_boundary() {
        let config = DecisionConfig::new(
            DecisionWeights::new(1.0, 0.0, 0.0).unwrap(),
            DecisionWeights::default(),
        );
        let just_below = evaluate_decision(
            &input(
                0.976,
                0.5,
                TrendRegime::Neutral,
                DecisionSentiment::Available(Sentiment::neutral()),
            ),
            &config,
        );
        let just_above = evaluate_decision(
            &input(
                0.974,
                0.5,
                TrendRegime::Neutral,
                DecisionSentiment::Available(Sentiment::neutral()),
            ),
            &config,
        );

        assert!(just_below.multiplier.value() < Multiplier::SKIP_BELOW.value());
        assert!(just_above.multiplier.value() > Multiplier::SKIP_BELOW.value());
        assert_eq!(just_below.action, Action::Skip);
        assert_eq!(just_above.action, Action::Underweight);
        assert_close(
            just_above.multiplier.value() - just_below.multiplier.value(),
            0.004,
        );
    }
}
