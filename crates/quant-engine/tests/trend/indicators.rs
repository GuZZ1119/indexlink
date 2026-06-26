use crate::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════
// 单指标隔离（验证方向约定）
// ═══════════════════════════════════════════════════════════════════════════

trend_deferred_test! {
fn ma_only_high_distance_produces_low_score() {
    // 仅 MA 权重=1.0：极高 MA 距离分位应映射为极低 score（反向）。
    // score = 1 − ma_p，当 ma_p → 1.0 时 score → 0.0。
    let history = standard_history();
    let snapshot = TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: 9999.0,
        rsi_history: history.clone(),
        rsi_current: 50.0, // 任意值，RSI 权重=0 不影响结果
        vix_history: history.clone(),
        vix_current: 50.0, // 任意值，VIX 权重=0 不影响结果
    };
    let config = trend_config_with_weights(TREND_MA_ONLY_MA, TREND_MA_ONLY_RSI, TREND_MA_ONLY_VIX);

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    assert_eq!(
        signal.score.value(),
        MIN_PERCENTILE,
        "MA 全权重且极高距离时 score 应精确为 0.0"
    );
}
}

trend_deferred_test! {
fn ma_only_score_equals_one_minus_ma_percentile() {
    // 仅 MA 权重=1.0，中间分位：score 应精确等于 1 − ma_p。
    let history = standard_history();
    let snapshot = TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: 50.0,
        rsi_history: history.clone(),
        rsi_current: 50.0,
        vix_history: history.clone(),
        vix_current: 50.0,
    };
    let config = trend_config_with_weights(TREND_MA_ONLY_MA, TREND_MA_ONLY_RSI, TREND_MA_ONLY_VIX);

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    let expected = 1.0 - signal.ma_distance_percentile.value();
    assert!(
        (signal.score.value() - expected).abs() < EXACT_FLOAT_TOLERANCE,
        "MA 全权重时 score 应精确等于 1 − ma_p，预期 {expected}，实际 {}",
        signal.score
    );
}
}

trend_deferred_test! {
fn rsi_only_high_rsi_produces_low_score() {
    // 仅 RSI 权重=1.0：极高 RSI 分位（超买）应映射为极低 score（反向）。
    let history = standard_history();
    let snapshot = TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: 50.0,
        rsi_history: history.clone(),
        rsi_current: 9999.0,
        vix_history: history.clone(),
        vix_current: 50.0,
    };
    let config =
        trend_config_with_weights(TREND_RSI_ONLY_MA, TREND_RSI_ONLY_RSI, TREND_RSI_ONLY_VIX);

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    assert_eq!(
        signal.score.value(),
        MIN_PERCENTILE,
        "RSI 全权重且极高超买时 score 应精确为 0.0"
    );
}
}

trend_deferred_test! {
fn rsi_only_score_equals_one_minus_rsi_percentile() {
    // 仅 RSI 权重=1.0，中间分位：score 应精确等于 1 − rsi_p。
    let history = standard_history();
    let snapshot = TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: 50.0,
        rsi_history: history.clone(),
        rsi_current: 70.0,
        vix_history: history.clone(),
        vix_current: 50.0,
    };
    let config =
        trend_config_with_weights(TREND_RSI_ONLY_MA, TREND_RSI_ONLY_RSI, TREND_RSI_ONLY_VIX);

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    let expected = 1.0 - signal.rsi_percentile.value();
    assert!(
        (signal.score.value() - expected).abs() < EXACT_FLOAT_TOLERANCE,
        "RSI 全权重时 score 应精确等于 1 − rsi_p，预期 {expected}，实际 {}",
        signal.score
    );
}
}

trend_deferred_test! {
fn vix_only_high_vix_produces_high_score() {
    // 仅 VIX 权重=1.0：极高 VIX 分位（高恐慌）应映射为极高 score（正向，与 MA/RSI 相反）。
    let history = standard_history();
    let snapshot = TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: 50.0,
        rsi_history: history.clone(),
        rsi_current: 50.0,
        vix_history: history.clone(),
        vix_current: 9999.0,
    };
    let config =
        trend_config_with_weights(TREND_VIX_ONLY_MA, TREND_VIX_ONLY_RSI, TREND_VIX_ONLY_VIX);

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    assert_eq!(
        signal.score.value(),
        MAX_PERCENTILE,
        "VIX 全权重且极高恐慌时 score 应精确为 1.0"
    );
}
}

trend_deferred_test! {
fn vix_only_score_equals_vix_percentile() {
    // 仅 VIX 权重=1.0，中间分位：score 应精确等于 vix_p（正向，不反向）。
    let history = standard_history();
    let snapshot = TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: 50.0,
        rsi_history: history.clone(),
        rsi_current: 50.0,
        vix_history: history.clone(),
        vix_current: 30.0,
    };
    let config =
        trend_config_with_weights(TREND_VIX_ONLY_MA, TREND_VIX_ONLY_RSI, TREND_VIX_ONLY_VIX);

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    let expected = signal.vix_percentile.value();
    assert!(
        (signal.score.value() - expected).abs() < EXACT_FLOAT_TOLERANCE,
        "VIX 全权重时 score 应精确等于 vix_p（正向），预期 {expected}，实际 {}",
        signal.score
    );
}
}

trend_deferred_test! {
fn high_vix_with_neutral_ma_rsi_pushes_score_above_neutral() {
    // 高 VIX + 中性 MA/RSI：验证 VIX 正向计入会单独把 score 推高，
    // 与 fundamental 层「低 ERP 单独推高得分」的场景对称。
    let history = standard_history();
    let snapshot = TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: 50.5, // ≈ 中性
        rsi_history: history.clone(),
        rsi_current: 50.5,
        vix_history: history.clone(),
        vix_current: 9999.0, // 极高 VIX
    };
    let config = trend_balanced_test_config();

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    assert!(
        signal.score.value() > NEUTRAL_PERCENTILE,
        "高 VIX 应把 score 推高过中性 0.50，实际 {}",
        signal.score
    );
}
}

// ═══════════════════════════════════════════════════════════════════════════
// 审计字段（原始分位未反向）
// ═══════════════════════════════════════════════════════════════════════════

trend_deferred_test! {
fn audit_fields_reflect_raw_unreversed_percentiles() {
    // 三个审计分位须如实记录原始（未反向）分位，便于回放决策。
    // 当 MA/RSI 极高时，ma_distance_percentile 与 rsi_percentile 应接近 1.0（未反向）；
    // 当 VIX 极低时，vix_percentile 应接近 0.0。
    let snapshot = overheated_trend_snapshot();
    let config = trend_balanced_test_config();

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    assert_eq!(
        signal.ma_distance_percentile.value(),
        MAX_PERCENTILE,
        "赶顶场景下 ma_distance_percentile 应为 1.0（未反向）"
    );
    assert_eq!(
        signal.rsi_percentile.value(),
        MAX_PERCENTILE,
        "赶顶场景下 rsi_percentile 应为 1.0（未反向）"
    );
    assert_eq!(
        signal.vix_percentile.value(),
        MIN_PERCENTILE,
        "赶顶场景下 vix_percentile 应为 0.0（极低恐慌）"
    );
}
}

trend_deferred_test! {
fn audit_fields_falling_knife_scenario() {
    // 接飞刀场景：vix_percentile 极高，ma_distance_percentile 与 rsi_percentile 极低。
    let snapshot = falling_knife_trend_snapshot();
    let config = trend_balanced_test_config();

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    assert_eq!(
        signal.vix_percentile.value(),
        MAX_PERCENTILE,
        "接飞刀场景下 vix_percentile 应为 1.0（高恐慌，正向，未反向）"
    );
    assert_eq!(
        signal.ma_distance_percentile.value(),
        MIN_PERCENTILE,
        "接飞刀场景下 ma_distance_percentile 应为 0.0"
    );
    assert_eq!(
        signal.rsi_percentile.value(),
        MIN_PERCENTILE,
        "接飞刀场景下 rsi_percentile 应为 0.0"
    );
}
}

trend_deferred_test! {
fn score_is_distinct_from_raw_audit_fields_when_reversed() {
    // 当 MA 权重=1.0 且 MA 分位处于极高位时，score 应与 ma_distance_percentile 不同
    // （因为 score = 1 − ma_p，两者之和 ≈ 1.0，不相等），验证反向确实生效。
    let history = standard_history();
    let snapshot = TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: 9999.0,
        rsi_history: history.clone(),
        rsi_current: 50.0,
        vix_history: history.clone(),
        vix_current: 50.0,
    };
    let config = trend_config_with_weights(TREND_MA_ONLY_MA, TREND_MA_ONLY_RSI, TREND_MA_ONLY_VIX);

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    assert_ne!(
        signal.score.value(),
        signal.ma_distance_percentile.value(),
        "score 与 ma_distance_percentile 不同（反向已生效）"
    );
    assert!(
        (signal.score.value() + signal.ma_distance_percentile.value() - 1.0).abs()
            < EXACT_FLOAT_TOLERANCE,
        "score + ma_p 应精确等于 1.0（验证反向公式）"
    );
}
}
