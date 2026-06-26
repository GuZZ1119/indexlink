// 覆盖跨层共享的历史分位计算工具，包括边界、脏数据过滤与错误上下文。

mod common;

use common::{
    make_history, standard_history, DEFAULT_MIN_HISTORY_LEN, MAX_PERCENTILE, MIN_PERCENTILE,
    NEUTRAL_PERCENTILE, STANDARD_HISTORY_LEN, TEST_MIN_HISTORY_LEN,
};
use quant_engine::{percentile_of, weighted_percentile_of, EwPercentileConfig, QuantError};

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn percentile_of_median() {
    let h = standard_history();
    let p = percentile_of("T", &h, 50.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert!(
        (p.value() - NEUTRAL_PERCENTILE).abs() < 0.01,
        "中位数应约为 50%，实际 {}",
        p
    );
}

#[test]
fn percentile_of_at_minimum() {
    let h = standard_history();
    let p = percentile_of("T", &h, 1.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(p.value(), 0.01, "只有 1 个值 ≤ 1，分位应为 1%");
}

#[test]
fn percentile_of_at_maximum() {
    let h = standard_history();
    let p = percentile_of("T", &h, 100.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(p.value(), MAX_PERCENTILE, "所有值 ≤ 100，分位应为 100%");
}

#[test]
fn percentile_of_below_all_history() {
    let h = standard_history();
    let p = percentile_of("T", &h, 0.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(p.value(), MIN_PERCENTILE, "无值 ≤ 0，分位应为 0%");
}

#[test]
fn percentile_of_above_all_history() {
    let h = standard_history();
    let p = percentile_of("T", &h, 9999.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(p.value(), MAX_PERCENTILE, "所有值 ≤ 9999，分位应为 100%");
}

#[test]
fn insufficient_history_returns_error() {
    let h = vec![1.0, 2.0, 3.0];
    let err = percentile_of("CAPE", &h, 2.0, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(
        matches!(err, QuantError::InsufficientHistory { found: 3, .. }),
        "应返回 InsufficientHistory，实际 {:?}",
        err
    );
}

#[test]
fn nan_current_returns_error() {
    let h = standard_history();
    let err = percentile_of("ERP", &h, f64::NAN, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(matches!(
        err,
        QuantError::InvalidCurrentValue {
            indicator: "ERP",
            ..
        }
    ));
}

#[test]
fn zero_min_len_returns_error_before_empty_history_division() {
    // min_len = 0 曾会让空历史通过长度检查，随后 0/0 产生 NaN 并 panic。
    let h = Vec::new();
    let err = percentile_of("CAPE", &h, 10.0, 0).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidMinHistoryLen { value: 0 }),
        "min_len = 0 应在分位计算前被拒绝，实际 {:?}",
        err
    );
}

#[test]
fn nan_in_history_is_filtered() {
    let mut h: Vec<f64> = make_history(90);
    h.extend(std::iter::repeat_n(f64::NAN, TEST_MIN_HISTORY_LEN));
    let p = percentile_of("T", &h, 45.0, DEFAULT_MIN_HISTORY_LEN).unwrap();
    assert!(p.value() > MIN_PERCENTILE);
}

#[test]
fn duplicate_values_use_less_than_or_equal() {
    // 50 个 1.0 + 50 个 2.0；current = 1.0 时 "<=" 应计入全部并列值。
    let mut h = vec![1.0_f64; 50];
    h.extend(vec![2.0_f64; 50]);
    let p = percentile_of("T", &h, 1.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(
        p.value(),
        NEUTRAL_PERCENTILE,
        "50 个 ≤ 1.0 的并列值应得 50% 分位，实际 {}",
        p
    );
}

#[test]
fn ew_percentile_config_from_half_life_maps_to_alpha() {
    let cfg = EwPercentileConfig::from_half_life(2.0, TEST_MIN_HISTORY_LEN).unwrap();

    assert_close((1.0_f64 - cfg.alpha()).powf(2.0), 0.5, 1e-12);
    assert_eq!(cfg.min_len(), TEST_MIN_HISTORY_LEN);
}

#[test]
fn ew_percentile_config_rejects_invalid_half_life() {
    let err = EwPercentileConfig::from_half_life(0.0, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(matches!(err, QuantError::InvalidHalfLife { value: 0.0 }));

    let err = EwPercentileConfig::from_half_life(f64::NAN, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(matches!(err, QuantError::InvalidHalfLife { value } if value.is_nan()));
}

#[test]
fn ew_percentile_config_rejects_invalid_alpha_and_min_len() {
    let err = EwPercentileConfig::from_alpha(0.0, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(matches!(err, QuantError::InvalidDecay { alpha: 0.0 }));

    let err = EwPercentileConfig::from_alpha(1.1, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(matches!(err, QuantError::InvalidDecay { alpha: 1.1 }));

    let err = EwPercentileConfig::from_alpha(0.5, 0).unwrap_err();
    assert!(matches!(err, QuantError::InvalidMinHistoryLen { value: 0 }));
}

#[test]
fn weighted_percentile_is_monotonic_in_current_value() {
    let h = standard_history();
    let cfg = EwPercentileConfig::from_alpha(0.5, TEST_MIN_HISTORY_LEN).unwrap();

    let low = weighted_percentile_of("T", &h, 25.0, &cfg).unwrap();
    let high = weighted_percentile_of("T", &h, 75.0, &cfg).unwrap();

    assert!(
        low <= high,
        "current 增大时加权分位不应下降：low {} high {}",
        low,
        high
    );
}

#[test]
fn weighted_percentile_weights_recent_samples_more() {
    let cfg = EwPercentileConfig::from_alpha(0.5, 1).unwrap();

    let low_is_recent = vec![100.0, 0.0];
    let high_is_recent = vec![0.0, 100.0];

    let recent_low = weighted_percentile_of("T", &low_is_recent, 50.0, &cfg).unwrap();
    let recent_high = weighted_percentile_of("T", &high_is_recent, 50.0, &cfg).unwrap();

    assert_close(recent_low.value(), 2.0 / 3.0, 1e-12);
    assert_close(recent_high.value(), 1.0 / 3.0, 1e-12);
    assert!(
        recent_low > recent_high,
        "旧→新顺序应影响权重：最近低值应给出更高分位"
    );
}

#[test]
fn weighted_percentile_skips_nan_without_compressing_lag() {
    let h = vec![0.0, f64::NAN, 100.0];
    let cfg = EwPercentileConfig::from_alpha(0.5, 1).unwrap();

    let p = weighted_percentile_of("T", &h, 50.0, &cfg).unwrap();

    // 最新 100 权重 1.0；中间 NaN 跳过但仍占一个时间 lag；
    // 最旧 0 权重 0.25。因此分位 = 0.25 / (1.0 + 0.25) = 0.2。
    assert_close(p.value(), 0.2, 1e-12);
}

#[test]
fn weighted_percentile_softens_oldest_sample_drop() {
    let cfg = EwPercentileConfig::from_alpha(0.5, 1).unwrap();
    let with_old_low = vec![0.0, 100.0, 100.0, 100.0, 100.0, 100.0];
    let without_old_low = vec![100.0, 100.0, 100.0, 100.0, 100.0];

    let before = weighted_percentile_of("T", &with_old_low, 50.0, &cfg).unwrap();
    let after = weighted_percentile_of("T", &without_old_low, 50.0, &cfg).unwrap();

    assert!(
        (before.value() - after.value()).abs() < 0.05,
        "最旧端样本退出时，加权分位应只发生小幅变化：before {} after {}",
        before,
        after
    );
}

#[test]
fn weighted_percentile_propagates_invalid_current_and_insufficient_history() {
    let cfg = EwPercentileConfig::from_alpha(0.5, TEST_MIN_HISTORY_LEN).unwrap();

    let err = weighted_percentile_of("T", &standard_history(), f64::INFINITY, &cfg).unwrap_err();
    assert!(matches!(
        err,
        QuantError::InvalidCurrentValue { indicator: "T", .. }
    ));

    let err = weighted_percentile_of("T", &[1.0, 2.0, f64::NAN], 2.0, &cfg).unwrap_err();
    assert!(matches!(
        err,
        QuantError::InsufficientHistory {
            indicator: "T",
            found: 2,
            ..
        }
    ));
}

#[test]
fn weighted_percentile_returns_insufficient_when_all_valid_weights_underflow() {
    // alpha = 1.0 时，最新样本之后的历史权重都为 0。若最新样本是 NaN，
    // 旧端有效样本会通过有效样本数检查，但总有效权重仍为 0。
    let cfg = EwPercentileConfig::from_alpha(1.0, 1).unwrap();

    let err = weighted_percentile_of("T", &[42.0, f64::NAN], 50.0, &cfg).unwrap_err();

    assert_eq!(
        err,
        QuantError::InsufficientHistory {
            indicator: "T",
            required: 1,
            found: 0,
        }
    );
}

#[test]
fn all_nan_history_returns_insufficient() {
    // 全部为 NaN，过滤后有效数据为 0，应判定历史不足而非 panic。
    let h = vec![f64::NAN; STANDARD_HISTORY_LEN];
    let err = percentile_of("CAPE", &h, 10.0, DEFAULT_MIN_HISTORY_LEN).unwrap_err();
    assert!(
        matches!(err, QuantError::InsufficientHistory { found: 0, .. }),
        "全 NaN 历史应返回 InsufficientHistory{{found:0}}，实际 {:?}",
        err
    );
}

#[test]
fn exact_min_len_boundary_succeeds() {
    // 有效数据点数恰好等于 min_len 时应成功（边界包含）。
    let h = make_history(DEFAULT_MIN_HISTORY_LEN);
    let p = percentile_of("T", &h, 30.0, DEFAULT_MIN_HISTORY_LEN);
    assert!(p.is_ok(), "有效点数 == min_len 应成功，实际 {:?}", p);
}

#[test]
fn insufficient_history_reports_indicator_and_required() {
    // 校验错误携带完整上下文，供审计/降级链使用。
    let h = vec![1.0, 2.0, 3.0];
    let err = percentile_of("CAPE", &h, 2.0, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert_eq!(
        err,
        QuantError::InsufficientHistory {
            indicator: "CAPE",
            required: TEST_MIN_HISTORY_LEN,
            found: 3,
        }
    );
}

#[test]
fn positive_infinity_current_returns_error() {
    // 金融读数必须是有限数；+Inf 更像上游数据管道异常，而不是合法极端估值。
    let h = standard_history();
    let err = percentile_of("T", &h, f64::INFINITY, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidCurrentValue { indicator: "T", .. }),
        "+Inf 当前读数应返回 InvalidCurrentValue，实际 {:?}",
        err
    );
}

#[test]
fn negative_infinity_current_returns_error() {
    // 金融读数必须是有限数；-Inf 同样应被视为非法输入。
    let h = standard_history();
    let err = percentile_of("T", &h, f64::NEG_INFINITY, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidCurrentValue { indicator: "T", .. }),
        "-Inf 当前读数应返回 InvalidCurrentValue，实际 {:?}",
        err
    );
}

#[test]
fn quant_error_display_is_descriptive() {
    let insufficient = QuantError::InsufficientHistory {
        indicator: "CAPE",
        required: DEFAULT_MIN_HISTORY_LEN,
        found: 3,
    };
    assert_eq!(
        insufficient.to_string(),
        "CAPE: requires at least 60 historical data points, found 3"
    );

    let invalid = QuantError::InvalidCurrentValue {
        indicator: "ERP",
        value: f64::INFINITY,
    };
    assert_eq!(
        invalid.to_string(),
        "ERP current value must be finite, got inf"
    );

    let invalid_min_len = QuantError::InvalidMinHistoryLen { value: 0 };
    assert_eq!(
        invalid_min_len.to_string(),
        "min_history_len must be greater than 0, got 0"
    );

    let invalid_weight = QuantError::InvalidWeight { value: 1.5 };
    assert_eq!(
        invalid_weight.to_string(),
        "weight must be finite and in [0.0, 1.0], got 1.5"
    );

    let invalid_half_life = QuantError::InvalidHalfLife { value: 0.0 };
    assert_eq!(
        invalid_half_life.to_string(),
        "half_life must be finite and greater than 0, got 0"
    );

    let invalid_decay = QuantError::InvalidDecay { alpha: 1.1 };
    assert_eq!(
        invalid_decay.to_string(),
        "decay alpha must be finite and in (0.0, 1.0], got 1.1"
    );

    let invalid_threshold = QuantError::InvalidPercentileThreshold {
        name: "overheated_above",
        value: 1.5,
    };
    assert_eq!(
        invalid_threshold.to_string(),
        "overheated_above threshold must be finite and in [0.0, 1.0], got 1.5"
    );
}
