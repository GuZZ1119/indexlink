use crate::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════
// 过渡存根契约
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[allow(deprecated)]
fn stub_still_returns_neutral_score_for_transition_period() {
    // evaluate_trend_stub 在 evaluate_trend 落地前须保持原有中性契约，
    // 确保 Decision Engine 调用方在过渡期不产生方向性偏移。
    let signal = evaluate_trend_stub();
    assert_eq!(
        signal.score.value(),
        NEUTRAL_PERCENTILE,
        "过渡存根应返回中性 0.50，实际 {}",
        signal.score
    );
}

#[test]
#[allow(deprecated)]
fn stub_regime_is_neutral() {
    // 存根的节奏体制应为 Neutral，不应误触发 TacticalDelay。
    let signal = evaluate_trend_stub();
    assert_eq!(
        signal.regime,
        TrendRegime::Neutral,
        "过渡存根 regime 应为 Neutral"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 方向性
// ═══════════════════════════════════════════════════════════════════════════

trend_deferred_test! {
fn trend_overheated_market_score_is_low() {
    // 强势上涨/赶顶场景：MA 距离与 RSI 极高（分位→1.0），VIX 极低（分位→0.0）。
    // MA/RSI 反向计入 → score 趋 0；方向约定：0.0 = 赶顶。
    let snapshot = overheated_trend_snapshot();
    let config = trend_balanced_test_config();

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    assert!(
        signal.score.value() < CHEAP_SCORE_UPPER_BOUND,
        "赶顶场景得分应 < {CHEAP_SCORE_UPPER_BOUND:.2}，实际 {}",
        signal.score
    );
}
}

trend_deferred_test! {
fn trend_falling_knife_market_score_is_high() {
    // 急跌/接飞刀场景：VIX 极高（分位→1.0），MA 距离与 RSI 极低（分位→0.0）。
    // VIX 正向计入 → score 趋 1；方向约定：1.0 = 接飞刀。
    let snapshot = falling_knife_trend_snapshot();
    let config = trend_balanced_test_config();

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    assert!(
        signal.score.value() > EXPENSIVE_SCORE_LOWER_BOUND,
        "接飞刀场景得分应 > {EXPENSIVE_SCORE_LOWER_BOUND:.2}，实际 {}",
        signal.score
    );
}
}

trend_deferred_test! {
fn trend_neutral_market_score_is_near_half() {
    // 横盘中性场景：三指标均处于历史中位。
    let snapshot = neutral_trend_snapshot();
    let config = trend_balanced_test_config();

    let signal = evaluate_trend(&snapshot, &config).unwrap();
    assert!(
        (signal.score.value() - NEUTRAL_PERCENTILE).abs() < NEUTRAL_TOLERANCE,
        "中性场景得分应 ≈ 0.50，实际 {}",
        signal.score
    );
}
}
