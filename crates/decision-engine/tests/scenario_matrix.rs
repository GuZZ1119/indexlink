//! Frozen 70/20/10 decision scenarios for regression coverage.
//!
//! Historical rows are derived from a fixed 2020-01 to 2025-07 monthly sample
//! using read-only OpenD SPY closes plus public CAPE, Treasury and VIX inputs.
//! Historical Qwen output was not available at those dates, so every historical
//! row must remain on the documented 90/10/0 fallback. Current-path rows use
//! bounded, frozen Qwen scores to test the normal 70/20/10 branch without
//! making CI depend on credentials, news feeds, or a live model response.

use ai_client::Sentiment;
use core_domain::{Action, Percentile};
use decision_engine::{
    evaluate_decision, DecisionConfig, DecisionInput, DecisionSentiment, DecisionWeightMode,
};
use quant_engine::{FundamentalSignal, TrendRegime, TrendSignal};

// Fixtures retain six decimal places from the offline data-preparation step.
// Keep enough tolerance for that documented rounding, while still detecting a
// meaningful change in the decision formula or weight selection.
const EPSILON: f64 = 5e-6;

#[derive(Clone, Copy)]
struct Scenario {
    id: &'static str,
    as_of: &'static str,
    profile: &'static str,
    fundamental: f64,
    trend: f64,
    regime: TrendRegime,
    sentiment: Option<f64>,
    expected_score: f64,
    expected_multiplier: f64,
    expected_action: Action,
}

impl Scenario {
    #[allow(
        clippy::too_many_arguments,
        reason = "the fixture constructor keeps each historical scenario readable as one row"
    )]
    const fn historical(
        id: &'static str,
        as_of: &'static str,
        profile: &'static str,
        fundamental: f64,
        trend: f64,
        regime: TrendRegime,
        expected_score: f64,
        expected_multiplier: f64,
        expected_action: Action,
    ) -> Self {
        Self {
            id,
            as_of,
            profile,
            fundamental,
            trend,
            regime,
            sentiment: None,
            expected_score,
            expected_multiplier,
            expected_action,
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the fixture constructor keeps each Qwen scenario readable as one row"
    )]
    const fn current_qwen(
        id: &'static str,
        profile: &'static str,
        fundamental: f64,
        trend: f64,
        regime: TrendRegime,
        sentiment: f64,
        expected_score: f64,
        expected_multiplier: f64,
        expected_action: Action,
    ) -> Self {
        Self {
            id,
            as_of: "current-frozen-qwen",
            profile,
            fundamental,
            trend,
            regime,
            sentiment: Some(sentiment),
            expected_score,
            expected_multiplier,
            expected_action,
        }
    }
}

const HISTORICAL: [Scenario; 30] = [
    Scenario::historical(
        "H01",
        "2020-01",
        "student_usd500_initial_usd200_monthly",
        0.510365,
        0.556944,
        TrendRegime::Neutral,
        0.484977,
        0.969954,
        Action::Standard,
    ),
    Scenario::historical(
        "H02",
        "2020-03",
        "worker_usd5000_initial_usd600_monthly",
        0.029574,
        0.906268,
        TrendRegime::FallingKnife,
        0.882757,
        1.382760,
        Action::TacticalDelay,
    ),
    Scenario::historical(
        "H03",
        "2020-04",
        "family_usd30000_initial_usd2000_monthly",
        0.088554,
        0.770958,
        TrendRegime::FallingKnife,
        0.843206,
        1.343210,
        Action::TacticalDelay,
    ),
    Scenario::historical(
        "H04",
        "2020-06",
        "student_usd500_initial_usd200_monthly",
        0.245574,
        0.758540,
        TrendRegime::FallingKnife,
        0.703129,
        1.203130,
        Action::TacticalDelay,
    ),
    Scenario::historical(
        "H05",
        "2020-09",
        "worker_usd5000_initial_usd600_monthly",
        0.435077,
        0.465598,
        TrendRegime::Overheated,
        0.554991,
        1.054990,
        Action::TacticalDelay,
    ),
    Scenario::historical(
        "H06",
        "2021-01",
        "family_usd30000_initial_usd2000_monthly",
        0.690194,
        0.571155,
        TrendRegime::FallingKnife,
        0.321710,
        0.643420,
        Action::TacticalDelay,
    ),
    Scenario::historical(
        "H07",
        "2021-03",
        "student_usd500_initial_usd200_monthly",
        0.799954,
        0.361548,
        TrendRegime::Overheated,
        0.216196,
        0.432392,
        Action::TacticalDelay,
    ),
    Scenario::historical(
        "H08",
        "2021-06",
        "worker_usd5000_initial_usd600_monthly",
        0.743200,
        0.295292,
        TrendRegime::Neutral,
        0.260649,
        0.521298,
        Action::Underweight,
    ),
    Scenario::historical(
        "H09",
        "2021-09",
        "family_usd30000_initial_usd2000_monthly",
        0.784257,
        0.773767,
        TrendRegime::Neutral,
        0.216792,
        0.433584,
        Action::Underweight,
    ),
    Scenario::historical(
        "H10",
        "2021-11",
        "student_usd500_initial_usd200_monthly",
        0.765571,
        0.723834,
        TrendRegime::Neutral,
        0.238603,
        0.477205,
        Action::Underweight,
    ),
    Scenario::historical(
        "H11",
        "2022-01",
        "worker_usd5000_initial_usd600_monthly",
        0.796736,
        0.792339,
        TrendRegime::Neutral,
        0.203704,
        0.407407,
        Action::Underweight,
    ),
    Scenario::historical(
        "H12",
        "2022-03",
        "family_usd30000_initial_usd2000_monthly",
        0.752411,
        0.559633,
        TrendRegime::Neutral,
        0.266867,
        0.533734,
        Action::Underweight,
    ),
    Scenario::historical(
        "H13",
        "2022-06",
        "student_usd500_initial_usd200_monthly",
        0.473432,
        0.873070,
        TrendRegime::Neutral,
        0.486604,
        0.973208,
        Action::Standard,
    ),
    Scenario::historical(
        "H14",
        "2022-09",
        "worker_usd5000_initial_usd600_monthly",
        0.523894,
        0.943221,
        TrendRegime::Neutral,
        0.434173,
        0.868347,
        Action::Standard,
    ),
    Scenario::historical(
        "H15",
        "2022-10",
        "family_usd30000_initial_usd2000_monthly",
        0.515475,
        0.592902,
        TrendRegime::Neutral,
        0.476782,
        0.953565,
        Action::Standard,
    ),
    Scenario::historical(
        "H16",
        "2023-01",
        "student_usd500_initial_usd200_monthly",
        0.514148,
        0.399971,
        TrendRegime::Neutral,
        0.477264,
        0.954528,
        Action::Standard,
    ),
    Scenario::historical(
        "H17",
        "2023-03",
        "worker_usd5000_initial_usd600_monthly",
        0.434613,
        0.317837,
        TrendRegime::Neutral,
        0.540632,
        1.040630,
        Action::Standard,
    ),
    Scenario::historical(
        "H18",
        "2023-06",
        "family_usd30000_initial_usd2000_monthly",
        0.725223,
        0.152702,
        TrendRegime::Neutral,
        0.262570,
        0.525139,
        Action::Underweight,
    ),
    Scenario::historical(
        "H19",
        "2023-08",
        "student_usd500_initial_usd200_monthly",
        0.735527,
        0.296217,
        TrendRegime::Neutral,
        0.267647,
        0.535295,
        Action::Underweight,
    ),
    Scenario::historical(
        "H20",
        "2023-10",
        "worker_usd5000_initial_usd600_monthly",
        0.596302,
        0.659582,
        TrendRegime::Neutral,
        0.397370,
        0.794740,
        Action::Standard,
    ),
    Scenario::historical(
        "H21",
        "2024-01",
        "family_usd30000_initial_usd2000_monthly",
        0.803978,
        0.296000,
        TrendRegime::Neutral,
        0.206020,
        0.412040,
        Action::Underweight,
    ),
    Scenario::historical(
        "H22",
        "2024-04",
        "student_usd500_initial_usd200_monthly",
        0.830679,
        0.462173,
        TrendRegime::Neutral,
        0.198606,
        0.397212,
        Action::Underweight,
    ),
    Scenario::historical(
        "H23",
        "2024-07",
        "worker_usd5000_initial_usd600_monthly",
        0.862446,
        0.430334,
        TrendRegime::Neutral,
        0.166832,
        0.333664,
        Action::Underweight,
    ),
    Scenario::historical(
        "H24",
        "2024-09",
        "family_usd30000_initial_usd2000_monthly",
        0.792975,
        0.273840,
        TrendRegime::Overheated,
        0.213707,
        0.427413,
        Action::TacticalDelay,
    ),
    Scenario::historical(
        "H25",
        "2024-11",
        "student_usd500_initial_usd200_monthly",
        0.914098,
        0.319229,
        TrendRegime::Neutral,
        0.109235,
        0.218469,
        Action::Underweight,
    ),
    Scenario::historical(
        "H26",
        "2025-01",
        "worker_usd5000_initial_usd600_monthly",
        0.906322,
        0.403968,
        TrendRegime::Neutral,
        0.124707,
        0.249414,
        Action::Underweight,
    ),
    Scenario::historical(
        "H27",
        "2025-03",
        "family_usd30000_initial_usd2000_monthly",
        0.682696,
        0.732840,
        TrendRegime::Neutral,
        0.312290,
        0.624579,
        Action::Underweight,
    ),
    Scenario::historical(
        "H28",
        "2025-04",
        "student_usd500_initial_usd200_monthly",
        0.550454,
        0.760646,
        TrendRegime::Neutral,
        0.428527,
        0.857054,
        Action::Standard,
    ),
    Scenario::historical(
        "H29",
        "2025-06",
        "worker_usd5000_initial_usd600_monthly",
        0.757808,
        0.408583,
        TrendRegime::Neutral,
        0.258831,
        0.517662,
        Action::Underweight,
    ),
    Scenario::historical(
        "H30",
        "2025-07",
        "family_usd30000_initial_usd2000_monthly",
        0.925932,
        0.383444,
        TrendRegime::Neutral,
        0.105006,
        0.210011,
        Action::Underweight,
    ),
];

const CURRENT_QWEN: [Scenario; 10] = [
    Scenario::current_qwen(
        "Q01",
        "student_usd200_monthly_cautious_news",
        0.85,
        0.45,
        TrendRegime::Neutral,
        -0.6,
        0.215,
        0.43,
        Action::Underweight,
    ),
    Scenario::current_qwen(
        "Q02",
        "student_usd200_monthly_low_valuation",
        0.25,
        0.50,
        TrendRegime::Neutral,
        0.4,
        0.695,
        1.195,
        Action::Overweight,
    ),
    Scenario::current_qwen(
        "Q03",
        "worker_usd600_monthly_balanced_news",
        0.50,
        0.50,
        TrendRegime::Neutral,
        0.0,
        0.500,
        1.000,
        Action::Standard,
    ),
    Scenario::current_qwen(
        "Q04",
        "worker_usd600_monthly_positive_qwen_news",
        0.35,
        0.50,
        TrendRegime::Neutral,
        0.8,
        0.645,
        1.145,
        Action::Overweight,
    ),
    Scenario::current_qwen(
        "Q05",
        "family_usd2000_monthly_high_valuation",
        0.80,
        0.50,
        TrendRegime::Neutral,
        0.6,
        0.320,
        0.640,
        Action::Underweight,
    ),
    Scenario::current_qwen(
        "Q06",
        "family_usd2000_monthly_falling_knife",
        0.30,
        0.85,
        TrendRegime::FallingKnife,
        0.2,
        0.580,
        1.080,
        Action::TacticalDelay,
    ),
    Scenario::current_qwen(
        "Q07",
        "graduate_usd400_monthly_recovery_news",
        0.20,
        0.40,
        TrendRegime::Neutral,
        -0.2,
        0.680,
        1.180,
        Action::Overweight,
    ),
    Scenario::current_qwen(
        "Q08",
        "investor_usd5000_monthly_overheated_news",
        0.90,
        0.10,
        TrendRegime::Overheated,
        -0.8,
        0.100,
        0.200,
        Action::TacticalDelay,
    ),
    Scenario::current_qwen(
        "Q09",
        "etf_usd1000_monthly_mixed_qwen_news",
        0.65,
        0.60,
        TrendRegime::Neutral,
        0.1,
        0.380,
        0.760,
        Action::Standard,
    ),
    Scenario::current_qwen(
        "Q10",
        "retirement_usd3000_monthly_defensive_news",
        0.45,
        0.50,
        TrendRegime::Neutral,
        -0.4,
        0.515,
        1.015,
        Action::Standard,
    ),
];

fn percentile(value: f64) -> Percentile {
    Percentile::new(value).expect("frozen scenario percentiles are bounded")
}

fn input(case: Scenario) -> DecisionInput {
    DecisionInput {
        fundamental: FundamentalSignal {
            score: percentile(case.fundamental),
            cape_percentile: percentile(case.fundamental),
            erp_percentile: percentile(1.0 - case.fundamental),
        },
        trend: TrendSignal {
            score: percentile(case.trend),
            ma_distance_percentile: percentile(0.5),
            rsi_percentile: percentile(0.5),
            vix_percentile: percentile(0.5),
            regime: case.regime,
        },
        sentiment: case
            .sentiment
            .map_or(DecisionSentiment::Unavailable, |value| {
                DecisionSentiment::Available(
                    Sentiment::new(value).expect("frozen sentiment is bounded"),
                )
            }),
    }
}

fn assert_close(actual: f64, expected: f64, case: Scenario) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "{} {} {}: expected {expected}, got {actual}",
        case.id,
        case.as_of,
        case.profile,
    );
}

fn assert_historical(case: Scenario) {
    let result = evaluate_decision(&input(case), &DecisionConfig::default());
    assert_eq!(
        result.weight_mode,
        DecisionWeightMode::SentimentUnavailable,
        "{}",
        case.id
    );
    assert_eq!(result.sentiment_score, None, "{}", case.id);
    assert_close(result.weights.fundamental_weight.value(), 0.90, case);
    assert_close(result.weights.trend_weight.value(), 0.10, case);
    assert_close(result.weights.sentiment_weight.value(), 0.0, case);
    assert_close(result.final_score.value(), case.expected_score, case);
    assert_close(result.multiplier.value(), case.expected_multiplier, case);
    assert_eq!(result.action, case.expected_action, "{}", case.id);
}

fn assert_current_qwen(case: Scenario) {
    let result = evaluate_decision(&input(case), &DecisionConfig::default());
    assert_eq!(
        result.weight_mode,
        DecisionWeightMode::Normal,
        "{}",
        case.id
    );
    assert!(result.sentiment_score.is_some(), "{}", case.id);
    assert_close(result.weights.fundamental_weight.value(), 0.70, case);
    assert_close(result.weights.trend_weight.value(), 0.20, case);
    assert_close(result.weights.sentiment_weight.value(), 0.10, case);
    assert_close(result.final_score.value(), case.expected_score, case);
    assert_close(result.multiplier.value(), case.expected_multiplier, case);
    assert_eq!(result.action, case.expected_action, "{}", case.id);
}

macro_rules! historical_test {
    ($name:ident, $index:expr) => {
        #[test]
        fn $name() {
            assert_historical(HISTORICAL[$index]);
        }
    };
}

macro_rules! current_qwen_test {
    ($name:ident, $index:expr) => {
        #[test]
        fn $name() {
            assert_current_qwen(CURRENT_QWEN[$index]);
        }
    };
}

historical_test!(historical_student_usd500_usd200_covid_pre_crash_2020_01, 0);
historical_test!(historical_worker_usd5000_usd600_covid_crash_2020_03, 1);
historical_test!(historical_family_usd30000_usd2000_covid_recovery_2020_04, 2);
historical_test!(historical_student_usd500_usd200_reopening_2020_06, 3);
historical_test!(historical_worker_usd5000_usd600_tech_overheat_2020_09, 4);
historical_test!(historical_family_usd30000_usd2000_growth_bull_2021_01, 5);
historical_test!(historical_student_usd500_usd200_growth_bull_2021_03, 6);
historical_test!(historical_worker_usd5000_usd600_late_cycle_2021_06, 7);
historical_test!(historical_family_usd30000_usd2000_delta_drawdown_2021_09, 8);
historical_test!(historical_student_usd500_usd200_rate_shock_2021_11, 9);
historical_test!(historical_worker_usd5000_usd600_rate_hike_2022_01, 10);
historical_test!(historical_family_usd30000_usd2000_bear_market_2022_03, 11);
historical_test!(
    historical_student_usd500_usd200_inflation_drawdown_2022_06,
    12
);
historical_test!(historical_worker_usd5000_usd600_bear_low_2022_09, 13);
historical_test!(
    historical_family_usd30000_usd2000_recovery_start_2022_10,
    14
);
historical_test!(historical_student_usd500_usd200_ai_recovery_2023_01, 15);
historical_test!(historical_worker_usd5000_usd600_banking_stress_2023_03, 16);
historical_test!(historical_family_usd30000_usd2000_soft_landing_2023_06, 17);
historical_test!(historical_student_usd500_usd200_rate_volatility_2023_08, 18);
historical_test!(
    historical_worker_usd5000_usd600_october_pullback_2023_10,
    19
);
historical_test!(historical_family_usd30000_usd2000_early_bull_2024_01, 20);
historical_test!(historical_student_usd500_usd200_spring_pullback_2024_04, 21);
historical_test!(historical_worker_usd5000_usd600_summer_rotation_2024_07, 22);
historical_test!(
    historical_family_usd30000_usd2000_election_volatility_2024_09,
    23
);
historical_test!(
    historical_student_usd500_usd200_post_election_bull_2024_11,
    24
);
historical_test!(
    historical_worker_usd5000_usd600_rate_uncertainty_2025_01,
    25
);
historical_test!(
    historical_family_usd30000_usd2000_spring_drawdown_2025_03,
    26
);
historical_test!(historical_student_usd500_usd200_tariff_drawdown_2025_04, 27);
historical_test!(historical_worker_usd5000_usd600_recovery_2025_06, 28);
historical_test!(historical_family_usd30000_usd2000_midyear_bull_2025_07, 29);

current_qwen_test!(current_qwen_student_usd200_cautious_news, 0);
current_qwen_test!(current_qwen_student_usd200_low_valuation, 1);
current_qwen_test!(current_qwen_worker_usd600_balanced_news, 2);
current_qwen_test!(current_qwen_worker_usd600_positive_news, 3);
current_qwen_test!(current_qwen_family_usd2000_high_valuation, 4);
current_qwen_test!(current_qwen_family_usd2000_falling_knife, 5);
current_qwen_test!(current_qwen_graduate_usd400_recovery_news, 6);
current_qwen_test!(current_qwen_investor_usd5000_overheated_news, 7);
current_qwen_test!(current_qwen_etf_usd1000_mixed_news, 8);
current_qwen_test!(current_qwen_retirement_usd3000_defensive_news, 9);
