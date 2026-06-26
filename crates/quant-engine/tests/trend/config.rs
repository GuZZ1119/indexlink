use crate::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════
// 配置不变量（构造期校验）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rejects_weight_sum_not_equal_to_one() {
    // 三权重之和明显偏离 1.0（和 = 1.1）应在 TrendWeights::new 构造时被拒绝。
    let err = TrendWeights::new(0.5, 0.3, 0.3).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidWeight { .. }),
        "权重和 ≠ 1.0 应返回 InvalidWeight，实际 {:?}",
        err
    );
}

#[test]
fn accepts_weight_sum_exactly_one() {
    // 权重和刚好等于 1.0 应构造成功。
    TrendWeights::new(0.4, 0.3, 0.3).expect("权重和 = 1.0 应构造成功");
}

#[test]
fn accepts_weight_sum_within_tolerance() {
    // 权重和在 [1.0 ± WEIGHT_SUM_TOLERANCE] 内应构造成功（ε = 1e-9）。
    let drift = 0.5 * EXACT_FLOAT_TOLERANCE;
    TrendWeights::new(0.4, 0.3, 0.3 + drift).expect("权重和在容忍范围内应构造成功");
}

#[test]
fn rejects_weight_sum_beyond_tolerance() {
    // 权重和超出 WEIGHT_SUM_TOLERANCE 应返回 InvalidWeight。
    let drift = 2.0 * EXACT_FLOAT_TOLERANCE;
    let vix = 0.3 + drift;
    let expected_sum = 0.4 + 0.3 + vix;
    let err = TrendWeights::new(0.4, 0.3, vix).unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InvalidWeight { value }
                if (value - expected_sum).abs() < EXACT_FLOAT_TOLERANCE
        ),
        "权重和超出容忍范围应返回 InvalidWeight {{ value: {expected_sum} }}，实际 {:?}",
        err
    );
}

#[test]
fn rejects_individual_weight_above_one() {
    // 单权重 > 1.0 应在 Weight::new 内部被拦截。
    let err = TrendWeights::new(1.5, 0.0, 0.0).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidWeight { .. }),
        "单权重 > 1.0 应返回 InvalidWeight，实际 {:?}",
        err
    );
}

#[test]
fn rejects_individual_weight_below_zero() {
    let err = TrendWeights::new(-0.1, 0.6, 0.5).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidWeight { .. }),
        "单权重 < 0.0 应返回 InvalidWeight，实际 {:?}",
        err
    );
}

#[test]
fn rejects_zero_min_history_len() {
    // min_len = 0 应在 EwPercentileConfig 构造期被拒绝。
    let err = EwPercentileConfig::from_half_life(TREND_TEST_HALF_LIFE, 0).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidMinHistoryLen { value: 0 }),
        "min_len = 0 应返回 InvalidMinHistoryLen，实际 {:?}",
        err
    );
}

#[test]
fn rejects_invalid_overheated_threshold() {
    // overheated_above 超出 [0.0, 1.0] 应在 TrendConfig::new 被拒绝。
    let weights = TrendWeights::new(
        TREND_EQUAL_MA_WEIGHT,
        TREND_EQUAL_RSI_WEIGHT,
        TREND_EQUAL_VIX_WEIGHT,
    )
    .unwrap();
    let err = TrendConfig::new(
        weights,
        trend_test_percentile_config(),
        1.5, // 超界
        TREND_FALLING_KNIFE_ABOVE,
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InvalidPercentileThreshold {
                name: "overheated_above",
                ..
            }
        ),
        "overheated_above 超界应返回 InvalidPercentileThreshold，实际 {:?}",
        err
    );
}

#[test]
fn rejects_invalid_falling_knife_threshold() {
    // falling_knife_above 超出 [0.0, 1.0] 应在 TrendConfig::new 被拒绝。
    let weights = TrendWeights::new(
        TREND_EQUAL_MA_WEIGHT,
        TREND_EQUAL_RSI_WEIGHT,
        TREND_EQUAL_VIX_WEIGHT,
    )
    .unwrap();
    let err = TrendConfig::new(
        weights,
        trend_test_percentile_config(),
        TREND_OVERHEATED_ABOVE,
        1.5, // 超界
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InvalidPercentileThreshold {
                name: "falling_knife_above",
                ..
            }
        ),
        "falling_knife_above 超界应返回 InvalidPercentileThreshold，实际 {:?}",
        err
    );
}

#[test]
fn rejects_nan_overheated_threshold() {
    let weights = TrendWeights::new(
        TREND_EQUAL_MA_WEIGHT,
        TREND_EQUAL_RSI_WEIGHT,
        TREND_EQUAL_VIX_WEIGHT,
    )
    .unwrap();
    let err = TrendConfig::new(
        weights,
        trend_test_percentile_config(),
        f64::NAN,
        TREND_FALLING_KNIFE_ABOVE,
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InvalidPercentileThreshold {
                name: "overheated_above",
                value,
            } if value.is_nan()
        ),
        "overheated_above 为 NaN 时应返回 InvalidPercentileThreshold，实际 {:?}",
        err
    );
}

#[test]
fn rejects_negative_threshold() {
    let weights = TrendWeights::new(
        TREND_EQUAL_MA_WEIGHT,
        TREND_EQUAL_RSI_WEIGHT,
        TREND_EQUAL_VIX_WEIGHT,
    )
    .unwrap();
    let err = TrendConfig::new(
        weights,
        trend_test_percentile_config(),
        -0.1,
        TREND_FALLING_KNIFE_ABOVE,
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InvalidPercentileThreshold {
                name: "overheated_above",
                value: -0.1,
            }
        ),
        "负阈值应返回 InvalidPercentileThreshold，实际 {:?}",
        err
    );
}

#[test]
fn rejects_nan_weight() {
    let err = TrendWeights::new(f64::NAN, 0.5, 0.5).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidWeight { .. }),
        "NaN 权重应返回 InvalidWeight，实际 {:?}",
        err
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 默认配置契约（与设计文档约定对齐）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn default_weights_match_design_constants() {
    // 默认子权重：MA=0.4, RSI=0.3, VIX=0.3，文档约定。
    let config = TrendConfig::default();
    assert!(
        (config.weights.ma_weight.value() - 0.4).abs() < EXACT_FLOAT_TOLERANCE,
        "默认 MA 权重应为 0.4，实际 {}",
        config.weights.ma_weight
    );
    assert!(
        (config.weights.rsi_weight.value() - 0.3).abs() < EXACT_FLOAT_TOLERANCE,
        "默认 RSI 权重应为 0.3，实际 {}",
        config.weights.rsi_weight
    );
    assert!(
        (config.weights.vix_weight.value() - 0.3).abs() < EXACT_FLOAT_TOLERANCE,
        "默认 VIX 权重应为 0.3，实际 {}",
        config.weights.vix_weight
    );
}

#[test]
fn default_thresholds_match_design_constants() {
    // 默认过热/接飞刀阈值均为 0.90。
    let config = TrendConfig::default();
    assert!(
        (config.overheated_above.value() - TREND_OVERHEATED_ABOVE).abs() < EXACT_FLOAT_TOLERANCE,
        "默认 overheated_above 应为 {TREND_OVERHEATED_ABOVE}"
    );
    assert!(
        (config.falling_knife_above.value() - TREND_FALLING_KNIFE_ABOVE).abs()
            < EXACT_FLOAT_TOLERANCE,
        "默认 falling_knife_above 应为 {TREND_FALLING_KNIFE_ABOVE}"
    );
}

#[test]
fn default_config_min_len_matches_design() {
    // 默认最少历史长度 60（5 年月度数据，与基本面层同源）。
    let config = TrendConfig::default();
    assert_eq!(
        config.percentile_config.min_len(),
        DEFAULT_MIN_HISTORY_LEN,
        "默认 min_len 应为 {DEFAULT_MIN_HISTORY_LEN}（月频）"
    );
}

#[test]
fn default_half_life_matches_design() {
    // 默认半衰期 36 个月，alpha 应满足 α = 1 − 0.5^(1/36)。
    let config = TrendConfig::default();
    let expected_alpha = 1.0 - 0.5_f64.powf(1.0 / DEFAULT_HALF_LIFE_MONTHS);
    assert!(
        (config.percentile_config.alpha() - expected_alpha).abs() < EXACT_FLOAT_TOLERANCE,
        "默认半衰期 {DEFAULT_HALF_LIFE_MONTHS} 个月应映射为 alpha {expected_alpha:.6}，实际 {}",
        config.percentile_config.alpha()
    );
}

#[test]
fn default_percentile_config_matches_fundamental() {
    // 趋势层默认分位配置与基本面层同源（月频 H=36, min_len=60）。
    let trend = TrendConfig::default();
    let fundamental = FundamentalConfig::default();
    assert_eq!(
        trend.percentile_config.min_len(),
        fundamental.percentile_config.min_len(),
        "趋势层与基本面层 min_len 应一致"
    );
    assert!(
        (trend.percentile_config.alpha() - fundamental.percentile_config.alpha()).abs()
            < EXACT_FLOAT_TOLERANCE,
        "趋势层与基本面层 alpha 应一致"
    );
}
