use crate::prelude::*;

trend_deferred_test! {
fn propagates_insufficient_history_for_ma_distance() {
    // MA 历史不足时应向上传播错误，供熔断链触发默认 Skip。
    let config = trend_balanced_test_config(); // min_len = 10
    let snapshot = TrendSnapshot {
        ma_distance_history: make_history(3), // 3 < 10
        ma_distance_current: 2.0,
        rsi_history: standard_history(),
        rsi_current: 50.0,
        vix_history: standard_history(),
        vix_current: 50.0,
    };

    let err = evaluate_trend(&snapshot, &config).unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InsufficientHistory {
                indicator: "MA_DISTANCE",
                ..
            }
        ),
        "MA 历史不足应传播 InsufficientHistory，实际 {:?}",
        err
    );
}
}

trend_deferred_test! {
fn propagates_insufficient_history_for_rsi() {
    // RSI 历史不足时指向 RSI 的错误。
    let config = trend_balanced_test_config();
    let snapshot = TrendSnapshot {
        ma_distance_history: standard_history(),
        ma_distance_current: 50.0,
        rsi_history: make_history(3),
        rsi_current: 50.0,
        vix_history: standard_history(),
        vix_current: 50.0,
    };

    let err = evaluate_trend(&snapshot, &config).unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InsufficientHistory {
                indicator: "RSI",
                ..
            }
        ),
        "RSI 历史不足应传播 InsufficientHistory，实际 {:?}",
        err
    );
}
}

trend_deferred_test! {
fn propagates_insufficient_history_for_vix() {
    // VIX 历史不足时指向 VIX 的错误。
    let config = trend_balanced_test_config();
    let snapshot = TrendSnapshot {
        ma_distance_history: standard_history(),
        ma_distance_current: 50.0,
        rsi_history: standard_history(),
        rsi_current: 50.0,
        vix_history: make_history(3),
        vix_current: 50.0,
    };

    let err = evaluate_trend(&snapshot, &config).unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InsufficientHistory {
                indicator: "VIX",
                ..
            }
        ),
        "VIX 历史不足应传播 InsufficientHistory，实际 {:?}",
        err
    );
}
}

trend_deferred_test! {
fn propagates_invalid_current_value_for_nan_ma() {
    // MA 当前读数为 NaN 时应传播 InvalidCurrentValue，绝不静默产出得分。
    let config = trend_balanced_test_config();
    let snapshot = TrendSnapshot {
        ma_distance_history: standard_history(),
        ma_distance_current: f64::NAN,
        rsi_history: standard_history(),
        rsi_current: 50.0,
        vix_history: standard_history(),
        vix_current: 50.0,
    };

    let err = evaluate_trend(&snapshot, &config).unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InvalidCurrentValue {
                indicator: "MA_DISTANCE",
                ..
            }
        ),
        "MA NaN 应传播 InvalidCurrentValue，实际 {:?}",
        err
    );
}
}

trend_deferred_test! {
fn propagates_invalid_current_value_for_nan_rsi() {
    let config = trend_balanced_test_config();
    let snapshot = TrendSnapshot {
        ma_distance_history: standard_history(),
        ma_distance_current: 50.0,
        rsi_history: standard_history(),
        rsi_current: f64::NAN,
        vix_history: standard_history(),
        vix_current: 50.0,
    };

    let err = evaluate_trend(&snapshot, &config).unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InvalidCurrentValue {
                indicator: "RSI",
                ..
            }
        ),
        "RSI NaN 应传播 InvalidCurrentValue，实际 {:?}",
        err
    );
}
}

trend_deferred_test! {
fn propagates_invalid_current_value_for_nan_vix() {
    let config = trend_balanced_test_config();
    let snapshot = TrendSnapshot {
        ma_distance_history: standard_history(),
        ma_distance_current: 50.0,
        rsi_history: standard_history(),
        rsi_current: 50.0,
        vix_history: standard_history(),
        vix_current: f64::NAN,
    };

    let err = evaluate_trend(&snapshot, &config).unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InvalidCurrentValue {
                indicator: "VIX",
                ..
            }
        ),
        "VIX NaN 应传播 InvalidCurrentValue，实际 {:?}",
        err
    );
}
}

trend_deferred_test! {
fn propagates_invalid_current_value_for_infinite_inputs() {
    // ±Inf 更像上游数据管道异常，应被拒绝。
    let config = trend_balanced_test_config();
    let snapshot = TrendSnapshot {
        ma_distance_history: standard_history(),
        ma_distance_current: f64::INFINITY,
        rsi_history: standard_history(),
        rsi_current: 50.0,
        vix_history: standard_history(),
        vix_current: 50.0,
    };

    let err = evaluate_trend(&snapshot, &config).unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InvalidCurrentValue {
                indicator: "MA_DISTANCE",
                ..
            }
        ),
        "Inf 当前读数应传播 InvalidCurrentValue，实际 {:?}",
        err
    );
}
}

trend_deferred_test! {
fn unequal_history_lengths_each_evaluated_independently() {
    // 三个历史序列长度不同：只要各自满足 min_len，应各自独立计算分位并成功。
    // 如未来加入等长校验，须改为断言返回错误。
    let config = trend_balanced_test_config(); // min_len = 10
    let snapshot = TrendSnapshot {
        ma_distance_history: standard_history(), // 100 点
        ma_distance_current: 50.0,
        rsi_history: make_history(20), // 20 点
        rsi_current: 10.0,
        vix_history: make_history(15), // 15 点
        vix_current: 8.0,
    };

    // 应成功，各自独立评估
    let result = evaluate_trend(&snapshot, &config);
    assert!(
        result.is_ok(),
        "不等长但各自满足 min_len 时应成功，实际 {:?}",
        result
    );
}
}
